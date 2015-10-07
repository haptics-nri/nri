extern crate time;
extern crate uuid;
extern crate rustc_serialize as serialize;

use std::sync::mpsc;
use std::collections::BTreeMap;
use ::teensy::ParkState;
use ::comms::CmdFrom;
use super::{ws_send, ws_rpc};
use self::uuid::Uuid;
use self::serialize::json::{ToJson, Json};

#[derive(Debug)]
enum EventContour {
    Starting,
    Continuing,
    Finishing
}


/// Descriptor of a data collection flow
pub struct Flow {
    /// Name of the flow
    pub name: String,
    /// States in the flow
    states: Vec<FlowState>,
    /// Is this the active flow?
    active: bool,
    /// All states done but one?
    almostdone: bool,

    stamp: Option<time::Timespec>,
    id: Option<uuid::Uuid>,
}

/// One state in a data collection flow
pub struct FlowState {
    /// Name of the flow state
    name: String,
    /// State of parking lot that allows this state (if applicable)
    park: Option<ParkState>,
    /// Commands to run for this state
    script: Vec<FlowCmd>,
    /// Has this state been completed?
    done: bool,
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
    Start(String),
    Stop(String),
    Send(String),
    StopSensors,
}

impl Flow {
    pub fn new(name: String, states: Vec<FlowState>) -> Flow {
        Flow { name: name, active: false, almostdone: false, states: states, stamp: None, id: None }
    }

    pub fn run(&mut self, park: ParkState, tx: &mpsc::Sender<CmdFrom>, wsid: usize) -> EventContour {
        let mut ret = EventContour::Continuing;

        // are we just starting the flow now?
        if !self.active {
            println!("Beginning flow {}!", self.name);

            // need a timestamp and ID
            self.stamp = Some(time::get_time());
            self.id = Some(uuid::Uuid::new_v4());
            self.active = true;

            ret = EventContour::Starting;
        }

        // find the next eligible state
        if let Some(state) = self.states.iter_mut().find(|s| !s.done && s.park.as_ref().map_or(true, |p| *p == park)) {
            println!("Executing state {}", state.name);
            state.run(tx, wsid);
            println!("Finished executing state {}", state.name);
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
                self.states.iter_mut().map(|s| { s.done = false; }).count();

                ret = EventContour::Finishing;
            } else {
                self.almostdone = true;
            }
        }

        ret
    }
}

impl FlowState {
    pub fn new(name: String, park: Option<ParkState>, script: Vec<FlowCmd>) -> FlowState {
        FlowState { name: name, park: park, script: script, done: false }
    }

    pub fn run(&mut self, tx: &mpsc::Sender<CmdFrom>, wsid: usize) {
        assert!(!self.done);
        for c in &mut self.script {
            c.run(&tx, wsid);
        }
        self.done = true;
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
            FlowCmd::Message(ref msg) => ws_send(wsid, format!("msg {}", msg)),
            FlowCmd::Str { ref prompt, ref mut data } => {
                assert!(data.is_none());
                *data = Some(ws_rpc(wsid,
                                    format!("Please enter {}: <form><input type=\"text\" name=\"{}\"/></form>",
                                            prompt, prompt),
                                    |x| {
                                        if x.is_empty() {
                                            Err("That's an empty string!".to_owned())
                                        } else {
                                            Ok(x)
                                        }
                                    }));
            }
            FlowCmd::Int { ref prompt, limits: (low, high), ref mut data } => {
                assert!(data.is_none());
                *data = Some(ws_rpc(wsid,
                                    format!("Please select {} ({}-{} scale): <form><input type=\"text\" name=\"{}\"/></form>",
                                            prompt, low, high, prompt),
                                    |x| {
                                        match x.parse() {
                                            Ok(i) if i >= low && i <= high => {
                                                Ok(i)
                                            }
                                            Ok(_) => {
                                                Err("Out of range!".to_owned())
                                            }
                                            Err(_) => {
                                                Err("Not an integer!".to_owned())
                                            }
                                        }
                                    }));
            }
            FlowCmd::Start(ref service) => {
                assert!(rpc!(tx, CmdFrom::Start, service.clone()).unwrap());
            }
            FlowCmd::Stop(ref service) => {
                assert!(rpc!(tx, CmdFrom::Stop, service.clone()).unwrap());
            }
            FlowCmd::Send(ref string) => {
                tx.send(CmdFrom::Data(string.clone())).unwrap();
            }
            FlowCmd::StopSensors => {
                for &svc in &["bluefox", "structure", "biotac", "optoforce", "teensy"] {
                    assert!(rpc!(tx, CmdFrom::Stop, svc.to_owned()).unwrap());
                }
            }
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

