use std::sync::mpsc;
use std::collections::BTreeMap;
use std::io::{Write, BufRead};
use std::fs::{self, File};
use std::path::PathBuf;
use std::process::Command;
use std::{fmt, env, thread};
use std::time::Duration;
use chrono::{DateTime, Local, Timelike};
use teensy::ParkState;
use comms::CmdFrom;
use super::ws;
use uuid::Uuid;
use serialize::json::{ToJson, Json};

#[derive(Debug)]
pub enum EventContour {
    Starting,
    Continuing,
    Finishing,
    In,
}

struct StampPrinter(DateTime<Local>);

impl fmt::Display for StampPrinter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.9}", self.0.timestamp() as f64 + self.0.nanosecond() as f64 / 1_000_000_000f64)
    }
}

/// Descriptor of a data collection flow
pub struct Flow {
    /// Name of the flow
    pub name: String, pub shortname: String,
    /// States in the flow
    states: Vec<FlowState>,
    /// Is this the active flow?
    active: bool,
    /// All states done but one?
    almostdone: bool,

    stamp: Option<DateTime<Local>>,
    dir: Option<PathBuf>,
    id: Option<Uuid>,
}

/// One state in a data collection flow
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
#[derive(Debug)]
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
    pub fn new(name: String, shortname: String, states: Vec<FlowState>) -> Flow {
        Flow { name: name, shortname: shortname, active: false, almostdone: false, states: states, stamp: None, dir: None, id: None }
    }

    pub fn run(&mut self, park: ParkState, tx: &mpsc::Sender<CmdFrom>, wsid: usize) -> EventContour {
        let mut ret = EventContour::In;

        // are we just starting the flow now?
        if !self.active {
            println!("Beginning flow {}!", self.name);

            // need a timestamp and ID
            self.stamp = Some(Local::now());
            self.id = Some(Uuid::new_v4());
            self.active = true;

            let datedir = self.stamp.unwrap().format("%Y%m%d").to_string();
            let fldir = format!("data/{}/{}", datedir, self.shortname); // TODO use PathBuf::push
            fs::create_dir_all(&fldir).unwrap();
            self.dir = Some(env::current_dir().unwrap());
            env::set_current_dir(&fldir).unwrap();
            let mut epnum = 1;
            for entry in fs::read_dir(".").unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir()
                {
                    let name = entry.file_name().into_string().unwrap();
                    let num = name.parse::<u64>().unwrap() + 1;
                    if num > epnum {
                        epnum = num;
                    }
                }
            }
            fs::create_dir(epnum.to_string()).unwrap();
            env::set_current_dir(epnum.to_string()).unwrap();

            ret = EventContour::Starting;
        }

        // find the next eligible state (if there is one)
        if let Some(state) = self.states.iter_mut().skip_while(|s| s.done).next() {
            if state.park.map_or(true, |p| p == park) {
                ret = EventContour::Continuing;
                println!("Executing state {}", state.name);
                state.run(tx, wsid);
                println!("Finished executing state {}", state.name);
            } else if state.park.is_some() {
                println!("Waiting for parking lot state to be {:?} (currently {:?})", state.park, park);
                ws::send(wsid, format!("msg Please insert {:?} end-effector", state.park.unwrap()));
            }
        } else {
            println!("No applicable states.");
        }

        let almostdone = match self.states.last() {
            Some(state) if state.done => true,
            _ => false,
        };
        if almostdone {
            if self.almostdone {
                // the flow is over! clear everything!

                let mut file = File::create(format!("{}.flow", self.shortname)).unwrap();
                writeln!(file, "{} [{}]", self.name, StampPrinter(self.stamp.unwrap())).unwrap();
                writeln!(file, "").unwrap();

                self.active = false;
                self.almostdone = false;
                for state in &mut self.states {
                    state.finalize(&mut file);
                    writeln!(file, "").unwrap();
                }

                env::set_current_dir(self.dir.take().unwrap()).unwrap();

                ret = EventContour::Finishing;
            } else {
                self.almostdone = true;
            }
        }

        ret
    }

    pub fn parse<R: BufRead>(shortname: String, reader: R) -> Result<Flow, (u32, &'static str)> {
        let lines = try!(reader.lines()
                               .collect::<Result<Vec<_>,_>>()
                               .map_err(|_| (0, "I/O error")));
        let mut lines = lines.into_iter()
                               .map(|s| s.trim_right().to_owned())
                               .peekable();
                          
        let name = try!(lines.find(|s| !s.is_empty()).ok_or((0, "Empty file")));
        let mut states = vec![];
        
        let mut i = 0;
        while lines.peek().is_some() {
            i += 1;
            let header = lines.next().unwrap();
            
            if header.is_empty() { continue; }
            let header = header.split("=>").collect::<Vec<_>>();
            if header.len() > 2 { return Err((i, "too many arrows")); }
            
            match header[0].chars().next() {
                Some('-') => {},
                Some(' ') => return Err((i, "command outside of state")),
                _ => return Err((i, "unindented line")),
            }
            
            let (name, park) = match header.len() {
                1 => { (header[0][1..].trim().to_owned(), None) },
                2 => { (header[1].trim().to_owned(),
                        Some(match header[0][1..].trim() {
                                ""          => ParkState::None,
                                "BioTac"    => ParkState::BioTac,
                                "OptoForce" => ParkState::OptoForce,
                                "Stick"     => ParkState::Stick,
                                _           => return Err((i, "bad trigger")),
                            })
                        )
                      },
                _ => unreachable!(),
            };
            
            let mut script = vec![];
            
            while lines.peek().is_some() && lines.peek().unwrap().starts_with(' ') {
                i += 1;
                let line = lines.next().unwrap();
                let line = line.trim();
                
                if        line.starts_with(':') {
                    script.push((FlowCmd::Send(line[1..].trim().to_owned()), None));
                } else if line.starts_with('"') {
                    script.push((FlowCmd::Message(line[1..line.len()-1].to_owned()), None));
                } else if line.starts_with('>') {
                    let nquotes = line.replace("\\\"", "").matches('"').count();
                    if nquotes == 1 { return Err((i, "unterminated string")); }
                    if nquotes > 2 { return Err((i, "too many strings")); }
                    
                    let s = line[line.find('"').unwrap()+1 .. line.rfind('"').unwrap()].trim();
                    let range = line[line.rfind('"').unwrap()+1 ..].trim();
                    
                    if range.len() > 0 {
                        if !(   range.chars().next() == Some('(')
                             && range.chars().rev().next() == Some(')')) {
                            return Err((i, "range not in parentheses"));
                        }
                        let dots = try!(range.find("..").ok_or((i, "not enough dots in range")));
                        let low = try!(range[1..dots].parse().map_err(|_| (i, "bad start of range")));
                        let high = try!(range[dots+2..range.len()-1].parse().map_err(|_| (i, "bad end of range")));
                        
                        script.push((FlowCmd::int(s.to_owned(), (low, high)), None));
                    } else {
                        script.push((FlowCmd::str(s.to_owned()), None));
                    }
                } else {
                    let mut words = line.split(' ').map(str::trim);
                    match words.next() {
                        None | Some("") => return Err((i, "empty command")),
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
                                script.push((FlowCmd::Start(split.next().unwrap().to_owned(), split.next().map(|s| s.to_owned())), None));
                            }
                        },
                        Some(_) => return Err((i, "invalid command")),
                    }
                }
            }
            
            states.push(FlowState::new(name, park, script));
        }
        
        Ok(Flow::new(name, shortname, states))
    }

}

impl FlowState {
    pub fn new(name: String, park: Option<ParkState>, script: Vec<(FlowCmd, Option<DateTime<Local>>)>) -> FlowState {
        FlowState { name: name, park: park, script: script, stamp: None, done: false }
    }

    pub fn run(&mut self, tx: &mpsc::Sender<CmdFrom>, wsid: usize) {
        self.stamp = Some(Local::now());
        for &mut (ref mut c, ref mut stamp) in &mut self.script {
            *stamp = Some(Local::now());
            c.run(&tx, wsid);
        }
        self.done = true;
    }

    pub fn finalize(&mut self, file: &mut File) {
        writeln!(file, "- {} [{}]", self.name, StampPrinter(self.stamp.unwrap())).unwrap();
        for &mut (ref mut c, ref mut stamp) in &mut self.script {
            write!(file, "    ").unwrap();
            c.finalize(file);
            writeln!(file, " [{}]", StampPrinter(stamp.unwrap())).unwrap();
            *stamp = None;
        }
        self.done = false;
        self.stamp = None;
    }
}

impl FlowCmd {
    pub fn str(prompt: String) -> FlowCmd {
        FlowCmd::Str { prompt: prompt, data: None }
    }

    pub fn int(prompt: String, limits: (i32, i32)) -> FlowCmd {
        FlowCmd::Int { prompt: prompt, limits: limits, data: None }
    }

    pub fn run(&mut self, tx: &mpsc::Sender<CmdFrom>, wsid: usize) {
        match *self {
            FlowCmd::Message(ref msg) => ws::send(wsid, format!("msg {}", msg)),
            FlowCmd::Str { ref prompt, ref mut data } => {
                assert!(data.is_none());
                *data = Some(ws::rpc(wsid,
                                    format!("prompt Please enter {}",
                                            prompt),
                                    |x| {
                                        if x.is_empty() {
                                            Err("prompt That's an empty string!".to_owned())
                                        } else {
                                            Ok(x)
                                        }
                                    }));
            }
            FlowCmd::Int { ref prompt, limits: (low, high), ref mut data } => {
                assert!(data.is_none());
                *data = Some(ws::rpc(wsid,
                                    format!("prompt Please select {} ({}-{} scale)",
                                            prompt, low, high),
                                    |x| {
                                        match x.parse() {
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
                                    }));
            }
            FlowCmd::Start(ref service, ref data) => {
                println!("Flow starting service {}", service);
                assert!(rpc!(tx, CmdFrom::Start, service.clone(), data.clone()).unwrap());
                println!("Flow waiting for service {} to start", service);
                thread::sleep(Duration::from_millis(2000));
                println!("Flow done waiting for service {}", service);
            }
            FlowCmd::Stop(ref service) => {
                assert!(rpc!(tx, CmdFrom::Stop, service.clone()).unwrap());
            }
            FlowCmd::Send(ref string) => {
                tx.send(CmdFrom::Data(String::from("to ") + string)).unwrap();
            }
            FlowCmd::StopSensors => {
                for &svc in &["bluefox", "structure", "biotac", "optoforce", "teensy"] {
                    assert!(rpc!(tx, CmdFrom::Stop, svc.to_owned()).unwrap());
                }
            }
        }
    }
    
    pub fn finalize(&mut self, file: &mut File) {
        match *self {
            FlowCmd::Message(ref msg) => write!(file, "{:?}", msg).unwrap(),
            FlowCmd::Str { ref prompt, ref mut data } => {
                write!(file, "> {:?} [{:?}]", prompt, data.as_ref().unwrap()).unwrap();
                *data = None;
            },
            FlowCmd::Int { ref prompt, limits: (low, high), ref mut data } => {
                write!(file, "> {:?} ({}..{}) [{:?}]", prompt, low, high, data.unwrap()).unwrap();
                *data = None;
            },
            FlowCmd::Start(ref service, ref data) => {
                write!(file, "start {}", service).unwrap();
                if let &Some(ref data) = data {
                    write!(file, "/{}", data).unwrap();
                }
            },
            FlowCmd::Stop(ref service) => write!(file, "stop {}", service).unwrap(),
            FlowCmd::Send(ref string) => write!(file, ": {}", string).unwrap(),
            FlowCmd::StopSensors => write!(file, "stop").unwrap(),
        }
    }
}

impl ToJson for Flow {
    fn to_json(&self) -> Json {
        let mut m: BTreeMap<String, Json> = BTreeMap::new();
        jsonize!(m, self; name, states, active, almostdone);
        m.to_json()
    }
}

impl ToJson for FlowState {
    fn to_json(&self) -> Json {
        let mut m: BTreeMap<String, Json> = BTreeMap::new();
        jsonize!(m, self; name, done);
        m.to_json()
    }
}

