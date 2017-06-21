#[macro_use] extern crate lazy_static;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate utils;
#[macro_use] extern crate comms;
extern crate teensy;
extern crate chrono;
extern crate uuid;

use std::{env, fmt, mem, thread};
use std::sync::mpsc;
use std::collections::HashMap;
use std::io::{Write, BufRead, BufReader};
use std::ffi::OsString;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::error::Error as StdError;
use std::sync::RwLock;
use std::time::Duration;
use chrono::{DateTime, Local, Timelike};
use teensy::ParkState;
use utils::config;
use comms::CmdFrom;
use uuid::Uuid;
#[macro_use] extern crate serde_derive;

error_chain! {
    errors {
        Io(action: String) {
            description("I/O error")
            display("I/O error while trying to {}", action)
        }

        ParseFlow(line: usize, error: &'static str) {
            description("syntax error in flow")
            display("flow syntax error: line {}: {}", line, error)
        }

        Rpc(action: String) {
            description("RPC error")
            display("RPC error while trying to {}", action)
        }

        FlowCanceled {}

        Comms {}
    }
}

trait OptionExt<T> {
    fn put(&mut self, it: T) -> &mut T;
}

impl<T> OptionExt<T> for Option<T> {
    fn put(&mut self, it: T) -> &mut T {
        *self = Some(it);
        self.as_mut().unwrap() // ok since the value is put in on the line above
    }
}

trait ResultExt2<T, E: StdError + Send + 'static> {
    fn comms_err(self) -> Result<T>;
}

impl<T, E: StdError + Send + 'static> ResultExt2<T, E> for StdResult<T, E> {
    fn comms_err(self) -> Result<T> {
        self.map_err(|err| Error::with_chain(err, ErrorKind::Comms))
    }
}

#[derive(Debug)]
struct OsStringError(OsString);

impl StdError for OsStringError {
    fn description(&self) -> &str {
        "invalid UTF-8"
    }
}

impl fmt::Display for OsStringError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

lazy_static! {
    pub static ref FLOWS: RwLock<HashMap<String, Flow>> = utils::watch(HashMap::new(),
                                                                       &FLOWS,
                                                                       Path::new(config::FLOW_PATH),
                                                                       "flow",
                                                                       |flows, path| {
        let flow = Flow::parse(path.file_stem().expect(&format!("no file stem for {:?}", path))
                                    .to_str().expect(&format!("bad UTF-8 in {:?}", path))
                                    .to_owned(),
                               BufReader::new(File::open(&path).expect(&format!("could not open flow {:?}", path))))
                        .expect(&format!("unable to parse flow {:?}", path));
        flows.insert(flow.shortname.clone(), flow);
    });

    pub static ref DATADIR: RwLock<String> = RwLock::new(utils::config::DATADIR.into());
}

pub trait Comms: Clone {
    type Error: StdError + Send + 'static;

    fn print(&self, msg: String) -> StdResult<(), Self::Error>;
    fn send(&self, msg: String) -> StdResult<(), Self::Error>;
    fn rpc<T, F: Fn(String) -> StdResult<T, String>>(&self, prompt: String, validator: F) -> StdResult<Option<T>, Self::Error>;
}

#[derive(Debug, PartialEq)]
pub enum EventContour {
    Starting,
    Continuing,
    Finishing,
    In,
}

struct StampPrinter<'a>(&'a DateTime<Local>);

impl<'a> fmt::Display for StampPrinter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.9}", self.0.timestamp() as f64 + self.0.nanosecond() as f64 / 1_000_000_000f64)
    }
}

/// Descriptor of a data collection flow
#[derive(Default, Serialize)]
pub struct Flow {
    /// Name of the flow
    pub name: String, pub shortname: String,
    pub endeffs: Vec<ParkState>,
    /// States in the flow
    states: Vec<FlowState>,
    /// Is this the active flow?
    active: bool,
    /// All states done but one?
    almostdone: bool,

    stamp: Option<DateTime<Local>>,
    original_dir: Option<PathBuf>,
    episode_dir: Option<PathBuf>,
    id: Option<Uuid>,

    #[serde(skip_serializing)]
    file: Option<File>,
}

impl Clone for Flow {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(), shortname: self.shortname.clone(),
            endeffs: self.endeffs.clone(),
            states: self.states.clone(),
            active: self.active,
            almostdone: self.almostdone,
            stamp: self.stamp.clone(),
            original_dir: self.original_dir.clone(),
            episode_dir: self.episode_dir.clone(),
            id: self.id.clone(),
            file: None,
        }
    }
}

/// One state in a data collection flow
#[derive(Clone, Serialize)]
pub struct FlowState {
    /// Name of the flow state
    name: String,
    /// State of parking lot that allows this state (if applicable)
    park: Option<ParkState>,
    /// Commands to run for this state
    script: Vec<(FlowCmd, Option<DateTime<Local>>)>,
    /// Has this state been completed?
    done: bool,
    stamp: Option<DateTime<Local>>,
}

/// Different actions that a flow can perform at each state
#[derive(Clone, Debug, Serialize)]
pub enum FlowCmd {
    Message(String),
    Str {
        prompt: String,
        data: Option<String>,
    },
    Int {
        prompt: String,
        limits: (i32, i32),
        data: Option<i32>,
    },
    Start(String, Option<String>),
    Stop(String),
    Send(String),
    StopSensors,
}

impl Flow {
    pub fn new(name: String, shortname: String, endeffs: Vec<ParkState>, states: Vec<FlowState>) -> Flow {
        Flow { name, shortname, endeffs, states, .. Flow::default() }
    }
    
    pub fn abort<C: Comms>(&mut self, tx: &mpsc::Sender<CmdFrom>, comms: C) -> Result<()> {
        use self::ErrorKind::*;
        // flow canceled! clear everything!

        comms.print("Aborting flow.".into()).comms_err()?;

        // reset state
        self.active = false;
        self.almostdone = false;
        let cap = Vec::with_capacity(self.states.len());
        let states = mem::replace(&mut self.states, cap);
        for mut state in states.into_iter().rev() {
            if state.done {
                comms.print(format!("\tCleaning up state \"{}\"", state.name)).comms_err()?;
                state.cleanup(tx, comms.clone())?;
            }
            let FlowState { name, park, script, .. } = state;
            self.states.push(FlowState::new(name, park, script));
        }
        self.states.reverse();

        // delete the files
        drop(self.file.take());
        if let (Some(original_dir), Some(episode_dir)) = (self.original_dir.take(), self.episode_dir.take()) {
            env::set_current_dir(&original_dir).chain_err(|| Io(format!("set current directory to {:?}", original_dir)))?;
            fs::remove_dir_all(&episode_dir).chain_err(|| Io(format!("delete directory {:?}", episode_dir)))?;
        }

        Ok(())
    }

    pub fn run<C: Comms>(&mut self, park: ParkState, tx: &mpsc::Sender<CmdFrom>, comms: C) -> Result<EventContour> {
        // TODO refactor this confusing function
        
        use self::ErrorKind::*;

        let mut ret = EventContour::In;

        // are we just starting the flow now?
        if !self.active {
            comms.print(format!("Beginning flow {}!", self.name)).comms_err()?;

            // need a timestamp and ID
            let stamp = self.stamp.put(Local::now());
            self.id = Some(Uuid::new_v4());
            self.active = true;

            let datedir = stamp.format("%Y%m%d").to_string();
            let fldir = format!("{}/{}/{}", *DATADIR.read().unwrap(), datedir, self.shortname); // TODO use PathBuf::push
            fs::create_dir_all(&fldir).chain_err(|| Io(format!("create episode directory {:?}", fldir)))?;
            self.original_dir = Some(env::current_dir().chain_err(|| Io("get current directory".into()))?);
            env::set_current_dir(&fldir).chain_err(|| Io(format!("set current directory to {:?}", fldir)))?;
            let mut epnum = 1;
            for entry in fs::read_dir(".").chain_err(|| Io("list current directory".into()))? {
                let entry = entry.chain_err(|| Io("read directory entry".into()))?;
                if entry.file_type().chain_err(|| Io(format!("read metadata of {:?}", entry.file_name())))?.is_dir() {
                    let name = entry.file_name().into_string().map_err(OsStringError).chain_err(|| Io(format!("invalid UTF-8 in dir name {:?}", entry.file_name())))?;
                    let num = 1 + name.parse::<u64>().chain_err(|| Io(format!("non-numeric directory {:?}", name)))?;
                    if num > epnum {
                        epnum = num;
                    }
                }
            }
            fs::create_dir(epnum.to_string()).chain_err(|| Io(format!("create directory \"{}\"", epnum)))?;
            env::set_current_dir(epnum.to_string()).chain_err(|| Io(format!("set current directory to \"{}\"", epnum)))?;
            self.episode_dir = Some(env::current_dir().chain_err(|| Io("get current directory".into()))?);

            // start off the flow file
            let shortname = &self.shortname;
            let file = self.file.put(File::create(format!("{}.flow", shortname)).chain_err(|| Io(format!("create flow file \"{}.flow\"", shortname)))?);
            writeln!(file, "{} [{}]", self.name, StampPrinter(stamp)).chain_err(|| Io(format!("write to flow file \"{}.flow\"", shortname)))?;
            writeln!(file, "").chain_err(|| Io(format!("write to flow file \"{}.flow\"", shortname)))?;

            ret = EventContour::Starting;
        }

        // find the next eligible state (if there is one)
        let mut abort = false;
        if let Some(state) = self.states.iter_mut().skip_while(|s| s.done).next() {
            if state.park.map_or(true, |p| p == park) {
                ret = EventContour::Continuing;
                comms.print(format!("Executing state {}", state.name)).comms_err()?;
                let file = self.file.as_mut().expect("flow file missing");
                let shortname = &self.shortname;
                match state.run(tx, file, comms.clone()) {
                    Ok(()) => (),
                    Err(Error(ErrorKind::FlowCanceled, ..)) => abort = true,
                    Err(e) => return Err(e)
                }
                writeln!(file, "").chain_err(|| Io(format!("write to flow file \"{}.flow\"", shortname)))?;
                comms.print(format!("Finished executing state {}", state.name)).comms_err()?;
            } else if let Some(goal) = state.park {
                comms.print(format!("Waiting for parking lot state to be {:?} (currently {:?})", goal, park)).comms_err()?;
                comms.send(format!("msg Please insert {:?} end-effector", goal)).comms_err()?;
            }
        } else {
            comms.print(format!("No applicable states.")).comms_err()?;
        }

        if abort {
            self.abort(tx, comms)?;
            return Ok(EventContour::Finishing);
        }

        let almostdone = match self.states.last() {
            Some(state) if state.done => true,
            _ => false,
        };
        if almostdone {
            if self.almostdone {
                // the flow is over! clear everything!

                self.active = false;
                self.almostdone = false;
                let mut states = mem::replace(&mut self.states, vec![]);
                for state in &mut states {
                    state.finalize()?;
                }
                self.states = states;

                if let Some(original_dir) = self.original_dir.take() {
                    env::set_current_dir(&original_dir).chain_err(|| Io(format!("set current directory to {:?}", original_dir)))?;
                }

                ret = EventContour::Finishing;
            } else {
                self.almostdone = true;
            }
        }

        Ok(ret)
    }

    pub fn parse<R: BufRead>(shortname: String, reader: R) -> Result<Flow> {
        use self::ErrorKind::*;

        let lines = reader.lines()
                          .collect::<StdResult<Vec<_>, _>>()
                          .chain_err(|| ParseFlow(0, "I/O error"))?;
        let mut lines = lines.into_iter()
                             .map(|s| s.trim_right().to_owned())
                             .peekable();
                          
        let header = lines.find(|s| !s.is_empty()).ok_or(ParseFlow(0, "Empty file"))?;
        let header = header.split(":").collect::<Vec<_>>();
        let (name, endeffs) = match header.len() {
            1 => (header[0].trim().to_owned(), vec![]),
            2 => (header[0].trim().to_owned(), header[1].trim().split(",").map(|s| s.trim().parse()).collect::<StdResult<Vec<_>, _>>().chain_err(|| ParseFlow(0, "invalid end-effector specifier"))?),
            _ => Err(ParseFlow(0, "bad header"))?
        };
        let mut states = vec![];
        
        let mut i = 0;
        while lines.peek().is_some() {
            i += 1;
            let header = lines.next().unwrap(); // ok because of the peek in the loop condition
            
            if header.is_empty() { continue; }
            let header = header.split("=>").collect::<Vec<_>>();
            if header.len() > 2 { Err(ParseFlow(i, "too many arrows"))?; }
            
            match header[0].chars().next() {
                Some('-') => {},
                Some(' ') => Err(ParseFlow(i, "command outside of state"))?,
                _ => Err(ParseFlow(i, "unindented line"))?,
            }
            
            let (name, park) = match header.len() {
                1 => { (header[0][1..].trim().to_owned(), None) },
                2 => { (header[1].trim().to_owned(),
                        Some(match header[0][1..].trim() {
                                ""          => ParkState::None,
                                "BioTac"    => ParkState::BioTac,
                                "OptoForce" => ParkState::OptoForce,
                                "Stick"     => ParkState::Stick,
                                _           => Err(ParseFlow(i, "bad trigger"))?,
                            })
                        )
                      },
                _ => unreachable!(), // due to early return above
            };
            
            let mut script = vec![];
            
            while lines.peek().is_some()
                  && lines.peek().unwrap().starts_with(' ') { // ok because of the peek
                i += 1;
                let line = lines.next().unwrap(); // ok because of the peek in the loop condition
                let line = line.trim();
                
                if        line.starts_with(':') {
                    script.push((FlowCmd::Send(line[1..].trim().to_owned()), None));
                } else if line.starts_with('"') {
                    script.push((FlowCmd::Message(line[1..line.len()-1].to_owned()), None));
                } else if line.starts_with('>') {
                    let q1 = line.find('"').ok_or(ParseFlow(i, "expected quoted string"))?;
                    let q2 = line.rfind('"').ok_or(ParseFlow(i, "unterminated string"))?;
                    let s = line[q1+1 .. q2].trim();
                    let range = line[q2+1 ..].trim();
                    
                    if !range.is_empty() {
                        if !range.starts_with('(') || !range.ends_with(')') {
                            Err(ParseFlow(i, "range not in parentheses"))?;
                        }
                        let dots = range.find("..").ok_or(ParseFlow(i, "not enough dots in range"))?;
                        let low = range[1..dots].parse().chain_err(|| ParseFlow(i, "bad start of range"))?;
                        let high = range[dots+2..range.len()-1].parse().chain_err(|| ParseFlow(i, "bad end of range"))?;
                        
                        script.push((FlowCmd::int(s.to_owned(), (low, high)), None));
                    } else {
                        script.push((FlowCmd::str(s.to_owned()), None));
                    }
                } else {
                    let mut words = line.split(' ').map(str::trim);
                    match words.next() {
                        None | Some("") => Err(ParseFlow(i, "empty command"))?,
                        Some("stop") => {
                            let mut words = words.peekable();
                            if words.peek().is_some() {
                                for word in words {
                                    script.push((FlowCmd::Stop(word.to_owned()), None));
                                }
                            } else {
                                script.push((FlowCmd::StopSensors, None));
                            }
                        },
                        Some("start") => {
                            for word in words {
                                let mut split = word.splitn(2, '/');
                                script.push((FlowCmd::Start(split.next().unwrap().to_owned(),
                                                // ok because splitn always returns at least one
                                             split.next().map(|s| s.to_owned())), None));
                            }
                        },
                        Some(_) => Err(ParseFlow(i, "invalid command"))?,
                    }
                }
            }
            
            states.push(FlowState::new(name, park, script));
        }
        
        Ok(Flow::new(name, shortname, endeffs, states))
    }

}

impl FlowState {
    pub fn new(name: String, park: Option<ParkState>, script: Vec<(FlowCmd, Option<DateTime<Local>>)>) -> FlowState {
        FlowState { name: name, park: park, script: script, stamp: None, done: false }
    }

    pub fn run<C: Comms>(&mut self, tx: &mpsc::Sender<CmdFrom>, file: &mut File, comms: C) -> Result<()> {
        use self::ErrorKind::*;

        let stamp = self.stamp.put(Local::now());
        writeln!(file, "- {} [{}]", self.name, StampPrinter(stamp)).chain_err(|| Io("write to flow file".into()))?;
        for &mut (ref mut c, ref mut stamp) in &mut self.script {
            *stamp = Some(Local::now());
            write!(file, "    ").chain_err(|| Io("write to flow file".into()))?;
            c.run(tx, file, comms.clone())?;
            writeln!(file, " [{}]", StampPrinter(stamp.as_ref().expect("missing timestamp"))).chain_err(|| Io("write to flow file".into()))?;
        }
        self.done = true;
        
        Ok(())
    }

    pub fn cleanup<C: Comms>(&mut self, tx: &mpsc::Sender<CmdFrom>, comms: C) -> Result<()> {
        for &mut (ref mut c, _) in self.script.iter_mut().rev() {
            comms.print(format!("\t\tUndoing {:?}", c)).comms_err()?;
            c.cleanup(tx)?;
        }

        Ok(())
    }

    pub fn finalize(&mut self) -> Result<()> {
        for &mut (ref mut c, ref mut stamp) in &mut self.script {
            c.finalize()?;
            *stamp = None;
        }
        self.done = false;
        self.stamp = None;

        Ok(())
    }
}

impl FlowCmd {
    pub fn str(prompt: String) -> FlowCmd {
        FlowCmd::Str { prompt: prompt, data: None }
    }

    pub fn int(prompt: String, limits: (i32, i32)) -> FlowCmd {
        FlowCmd::Int { prompt: prompt, limits: limits, data: None }
    }

    pub fn run<C: Comms>(&mut self, tx: &mpsc::Sender<CmdFrom>, file: &mut File, comms: C) -> Result<()> {
        use self::ErrorKind::*;

        match *self {
            FlowCmd::Message(ref msg) => {
                comms.send(format!("msg {}", msg)).comms_err()?;

                write!(file, "{:?}", msg).chain_err(|| Io("write to flow file".into()))?;
            }

            FlowCmd::Str { ref prompt, ref mut data } => {
                assert!(data.is_none());
                let data = data.put(comms.rpc(format!("prompt Please enter {}",
                                               prompt),
                                       |x| {
                                           if x.is_empty() {
                                               Err("prompt That's an empty string!".to_owned())
                                           } else {
                                               Ok(x)
                                           }
                                       }).comms_err()?
                                       .ok_or(ErrorKind::FlowCanceled)?);

                write!(file, "> {:?} [{:?}]", prompt, data).chain_err(|| Io("write to flow file".into()))?;
            }

            FlowCmd::Int { ref prompt, limits: (low, high), ref mut data } => {
                assert!(data.is_none());
                let data = data.put(comms.rpc(format!("prompt Please select {} ({}-{} scale)",
                                               prompt, low, high),
                                       |x| {
                                           match x.trim().parse() {
                                               Ok(i) if i >= low && i <= high => {
                                                   Ok(i)
                                               }
                                               Ok(_) => {
                                                   Err("prompt Out of range!".to_owned())
                                               }
                                               Err(_) => {
                                                   Err("prompt Not an integer!".to_owned())
                                               }
                                           }
                                       }).comms_err()?
                                       .ok_or(ErrorKind::FlowCanceled)?);

                write!(file, "> {:?} ({}..{}) [{:?}]", prompt, low, high, data).chain_err(|| Io("write to flow file".into()))?;
            }

            FlowCmd::Start(ref service, ref data) => {
                comms.print(format!("Flow starting service {}", service)).comms_err()?;
                rpc!(tx, CmdFrom::Start, service.clone(), data.clone()).chain_err(|| Rpc(format!("start {}", service)))?;
                comms.print(format!("Flow waiting for service {} to start", service)).comms_err()?;
                thread::sleep(Duration::from_millis(2000));
                comms.print(format!("Flow done waiting for service {}", service)).comms_err()?;

                write!(file, "start {}", service).chain_err(|| Io("write to flow file".into()))?;
                if let Some(ref data) = *data {
                    write!(file, "/{}", data).chain_err(|| Io("write to flow file".into()))?;
                }
            }

            FlowCmd::Stop(ref service) => {
                rpc!(tx, CmdFrom::Stop, service.clone()).chain_err(|| Rpc(format!("stop {}", service)))?;

                write!(file, "stop {}", service).chain_err(|| Io("write to flow file".into()))?;
            }

            FlowCmd::Send(ref string) => {
                tx.send(CmdFrom::Data(String::from("to ") + string)).chain_err(|| Rpc(format!("send to {}", string)))?;

                write!(file, ": {}", string).chain_err(|| Io("write to flow file".into()))?;
            }

            FlowCmd::StopSensors => {
                for &svc in &["bluefox", "structure", "biotac", "optoforce", "teensy"] {
                    rpc!(tx, CmdFrom::Stop, svc.to_owned()).chain_err(|| Rpc(format!("stop {}", svc)))?;
                }

                write!(file, "stop").chain_err(|| Io("write to flow file".into()))?;
            }
        }

        Ok(())
    }

    pub fn cleanup(&mut self, tx: &mpsc::Sender<CmdFrom>) -> Result<()> {
        use self::ErrorKind::*;

        match *self {
            FlowCmd::Start(ref service, _) => {
                // if a service was started, stop it
                rpc!(tx, CmdFrom::Stop, service.clone()).chain_err(|| Rpc(format!("[cleanup] stop {}", service)))?;
            }
            FlowCmd::Send(ref string) => {
                // if a "disk start" message was sent, send a "disk stop"
                if string.contains("disk start") {
                    let string = &string.replace("disk start", "disk stop");
                    tx.send(CmdFrom::Data(String::from("to ") + string)).chain_err(|| Rpc(format!("[cleanup] send to {}", string)))?;
                }
            }
            _ => { /* nothing to do */ }
        }

        Ok(())
    }
    
    pub fn finalize(&mut self) -> Result<()> {
        match *self {
            FlowCmd::Str { ref mut data, .. } => *data = None,
            FlowCmd::Int { ref mut data, .. } => *data = None,
            _ => {}
        }

        Ok(())
    }
}

