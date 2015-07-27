//! Web interface to view and control running services
//!
//! Uses the Iron web framework, Handlebars templates, and Twitter Boostrap.

extern crate iron;
extern crate handlebars_iron as hbs;
extern crate staticfile;
extern crate mount;
extern crate router;
extern crate hyper;
extern crate rustc_serialize as serialize;
extern crate websocket as ws;
extern crate uuid;
extern crate time;

use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;
use std::collections::BTreeMap;
use super::comms::{Controllable, CmdFrom, Block};
use super::stb::ParkState;
use self::iron::prelude::*;
use self::iron::status;
use self::iron::middleware::{Handler, AfterMiddleware};
use self::hbs::{Template, HandlebarsEngine};
#[cfg(feature = "watch")] use self::hbs::Watchable;
use self::serialize::json::{ToJson, Json};
use self::staticfile::Static;
use self::mount::Mount;
use self::router::Router;
use self::hyper::server::Listening;
use self::ws::Sender as WebsocketSender;
use self::ws::Receiver as WebsocketReceiver;
use self::uuid::Uuid;

static HTTP_PORT: u16 = 3000;
static WS_PORT: u16   = 3001;

/// Service descriptor
///
/// Unlike the one in main.rs, this descriptor only needs to contain things that are useful for
/// display in the interface. However, they should probably be unified (TODO). The "web descriptor"
/// could be just a subfield of the main.rs service descriptor, and then those could get passed in
/// here (somehow).
struct Service {
    name: String,
    shortname: String,
    extra: String
}

impl Service {
    /// Create a new service descriptor with the given name
    fn new(s: &str, t: &str, e: &str) -> Service {
        Service { name: s.to_string(), shortname: t.to_string(), extra: e.to_string() }
    }
}

/// Descriptor of a data collection flow
struct Flow {
    /// Name of the flow
    name: &'static str,
    /// States in the flow
    states: Vec<FlowState>,
    /// Is this the active flow?
    active: bool,

    stamp: Option<time::Timespec>,
    id: Option<uuid::Uuid>,
}

/// One state in a data collection flow
struct FlowState {
    /// Name of the flow state
    name: &'static str,
    /// State of parking lot that allows this state (if applicable)
    park: Option<ParkState>,
    /// Commands to run for this state
    script: Vec<FlowCmd>,
    /// Has this state been completed?
    done: bool,
}

/// Different actions that a flow can perform at each state
enum FlowCmd {
    Message(&'static str),
    Str { prompt: &'static str, data: Option<String> },
    Int { prompt: &'static str, limits: (i32, i32), data: Option<i32> },
    Start(&'static str),
    Stop(&'static str),
    Send(&'static str),
    StopSensors,
}

impl Flow {
    fn new(name: &'static str, states: Vec<FlowState>) -> Flow {
        Flow {
            name: name,
            states: states,
            active: false,

            stamp: None,
            id: None,
        }
    }

    fn run(&mut self, park: ParkState, tx: &mpsc::Sender<CmdFrom>, wstx: &mpsc::Sender<String>, wsrx: &mpsc::Receiver<String>) {
        // are we just starting the flow now?
        if !self.active {
            println!("Beginning flow {}!", self.name);

            // need a timestamp and ID
            self.stamp = Some(time::get_time());
            self.id = Some(uuid::Uuid::new_v4());
            self.active = true;
        }

        // find the next eligible state
        let state = self.states.iter_mut().find(|s| !s.done && s.park.as_ref().map_or(true, |p| *p == park)).unwrap();
        println!("Executing state {}", state.name);
        state.run(tx, wstx, wsrx);
        println!("Finished executing state {}", state.name);
    }
}

impl FlowState {
    fn new(name: &'static str, park: Option<ParkState>, script: Vec<FlowCmd>) -> FlowState {
        FlowState {
            name: name,
            park: park,
            script: script,
            done: false,
        }
    }
    
    fn run(&mut self, tx: &mpsc::Sender<CmdFrom>, wstx: &mpsc::Sender<String>, wsrx: &mpsc::Receiver<String>) {
        assert!(!self.done);
        self.script.iter_mut().map(|c| c.run(tx, wstx, wsrx)).count();
        self.done = true;
    }
}

impl FlowCmd {
    fn str(prompt: &'static str) -> FlowCmd {
        FlowCmd::Str { prompt: prompt, data: None}
    }

    fn int(prompt: &'static str, limits: (i32, i32)) -> FlowCmd {
        FlowCmd::Int { prompt: prompt, limits: limits, data: None}
    }

    fn run(&mut self, tx: &mpsc::Sender<CmdFrom>, wstx: &mpsc::Sender<String>, wsrx: &mpsc::Receiver<String>) {
        match *self {
            FlowCmd::Message(msg) => wstx.send(format!("{}", msg)).unwrap(),
            FlowCmd::Str { prompt, ref mut data } => {
                assert!(data.is_none());
                wstx.send(format!("Please enter {}: <form><input type=\"text\" name=\"{}\"/></form>", prompt, prompt)).unwrap();
                let mut answer = wsrx.recv().unwrap();
                while answer.is_empty() {
                    wstx.send(format!("That's an empty string! Please enter {}: <form><input type=\"text\" name=\"{}\"/></form>", prompt, prompt)).unwrap();
                    answer = wsrx.recv().unwrap();
                }
                *data = Some(answer);
            },
            FlowCmd::Int { prompt, limits: (low, high), ref mut data } => {
                assert!(data.is_none());
                wstx.send(format!("Please select {} ({}-{} scale): <form><input type=\"text\" name=\"{}\"/></form>", prompt, low, high, prompt)).unwrap();
                let mut answer = wsrx.recv().unwrap();
                loop {
                    match answer.parse() {
                        Ok(i) if i >= low && i <= high => {
                            *data = Some(i);
                            break;
                        },
                        Ok(_) => {
                            wstx.send(format!("Out of range! Please select {} ({}-{} scale): <form><input type=\"text\" name=\"{}\"/></form>", prompt, low, high, prompt)).unwrap();
                            answer = wsrx.recv().unwrap();
                        },
                        Err(_) => {
                            wstx.send(format!("Not an integer! Please select {} ({}-{} scale): <form><input type=\"text\" name=\"{}\"/></form>", prompt, low, high, prompt)).unwrap();
                            answer = wsrx.recv().unwrap();
                        },
                    }
                }
            },
            FlowCmd::Start(service) => {
                assert!(rpc!(tx, CmdFrom::Start, service.to_string()).unwrap());
            },
            FlowCmd::Stop(service) => {
                assert!(rpc!(tx, CmdFrom::Stop, service.to_string()).unwrap());
            },
            FlowCmd::Send(string) => {
                tx.send(CmdFrom::Data(string.to_string())).unwrap();
            },
            FlowCmd::StopSensors => {
                for svc in ["bluefox", "structure", "biotac", "optoforce", "stb"].into_iter() {
                    assert!(rpc!(tx, CmdFrom::Start, svc.to_string()).unwrap());
                }
            }
        }
    }
}

macro_rules! jsonize {
    ($map:ident, $selph:ident, $var:ident) => {{
        $map.insert(stringify!($var).to_string(), $selph.$var.to_json())
    }};
    ($map:ident, $selph:ident; $($var:ident),+) => {{
        $(jsonize!($map, $selph, $var));+
    }}
}

impl ToJson for Service {
    fn to_json(&self) -> Json {
        let mut m: BTreeMap<String, Json> = BTreeMap::new();
        jsonize!(m, self; name, shortname, extra);
        m.to_json()
    }
}

impl ToJson for Flow {
    fn to_json(&self) -> Json {
        let mut m: BTreeMap<String, Json> = BTreeMap::new();
        jsonize!(m, self; name, states);
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

/// Make a path relative to the current file's directory
fn relpath(path: &str) -> String {
    String::from(Path::new(file!()).parent().unwrap().join(path).to_str().unwrap())
}

/// Handler for the main page of the web interface
fn index(flows: Arc<RwLock<Vec<Flow>>>) -> Box<Handler> {
    Box::new(move |req: &mut Request| -> IronResult<Response> {
        let mut data = BTreeMap::<String, Json>::new();
        data.insert("services".to_string(), vec![ Service::new("Structure Sensor", "structure" , "<img class=\"structure latest\" src=\"img/structure_latest.png\" /><div class=\"structure framenum\">NaN</div>"),
                                                  Service::new("mvBlueFOX3"      , "bluefox"   , "<img class=\"bluefox latest\" src=\"img/bluefox_latest.png\" /><div class=\"bluefox framenum\">NaN</div>"),
                                                  Service::new("OptoForce"       , "optoforce" , ""),
                                                  Service::new("SynTouch BioTac" , "biotac"    , ""),
                                                  Service::new("STB"             , "stb"       , ""),
                                                ].to_json());
        data.insert("flows".to_string(), flows.read().unwrap().to_json());
        data.insert("server".to_string(), format!("{}:{}", req.url.host, WS_PORT).to_json());

        let mut resp = Response::new();
        resp.set_mut(Template::new("index", data)).set_mut(status::Ok);
        Ok(resp)
    })
}

/// Handler for starting/stopping a service
fn control(tx: mpsc::Sender<CmdFrom>) -> Box<Handler> {
    let mtx = Mutex::new(tx);
    Box::new(move |req: &mut Request| -> IronResult<Response> {
        let params = req.extensions.get::<Router>().unwrap();
        let service = params.find("service").unwrap();
        let action = params.find("action").unwrap();

        match action {
            "start" =>
                if rpc!(mtx.lock().unwrap(), CmdFrom::Start, service.to_string()).unwrap() {
                    Ok(Response::with((status::Ok, format!("Started {}", service))))
                } else {
                    Ok(Response::with((status::InternalServerError, format!("Failed to start {}", service))))
                },
            "stop" =>
                if rpc!(mtx.lock().unwrap(), CmdFrom::Stop, service.to_string()).unwrap() {
                    Ok(Response::with((status::Ok, format!("Stopped {}", service))))
                } else {
                    Ok(Response::with((status::InternalServerError, format!("Failed to stop {}", service))))
                },
            "kick" => 
                match mtx.lock().unwrap().send(CmdFrom::Data(format!("kick {}", service))) {
                    Ok(_) => Ok(Response::with((status::Ok, format!("Kicked {}", service)))),
                    Err(_) => Ok(Response::with((status::InternalServerError, format!("Failed to kick {}", service))))
                },
            _ => Ok(Response::with((status::BadRequest, format!("What does {} mean?", action))))
        }
    })
}

/// Handler for starting/continuing a flow
fn flow(tx: mpsc::Sender<CmdFrom>, flows: Arc<RwLock<Vec<Flow>>>) -> Box<Handler> {
    let mtx = Mutex::new(tx);
    Box::new(move |req: &mut Request| -> IronResult<Response> {
        let params = req.extensions.get::<Router>().unwrap();
        let service = params.find("flow").unwrap();
        let action = params.find("action").unwrap();

        match action {
            "start" => unimplemented!(),
            "continue" => unimplemented!(),
            _ => Ok(Response::with((status::BadRequest, format!("What does {} mean?", action))))
        }
    })
}

/// Controllable struct for the web server
pub struct Web {
    /// Private handle to the HTTP server
    listening: Listening,

    /// Private handle to the websocket server thread
    websocket: JoinHandle<()>,

    /// Private channel for sending events to WebSocket clients
    wstx: mpsc::Sender<ws::Message>,
}

struct Catchall;

impl Catchall {
    fn new() -> Catchall { Catchall }
}

impl AfterMiddleware for Catchall {
    fn catch(&self, req: &mut Request, err: IronError) -> IronResult<Response> {
        match err.response.status {
            Some(status::NotFound) => Ok(Response::with(status::NotFound)),
            _ => Err(err)
        }
    }
}

#[cfg(feature = "watch")]
fn maybe_watch(chain: &mut Chain) {
    let watcher = Arc::new(HandlebarsEngine::new(&relpath("templates"), ".hbs"));
    watcher.watch();
    chain.link_after(watcher);
}
#[cfg(not(feature = "watch"))]
fn maybe_watch(chain: &mut Chain) {
    chain.link_after(HandlebarsEngine::new(&relpath("templates"), ".hbs"));
}

guilty!{
    impl Controllable for Web {
        const NAME: &'static str = "web",
        const BLOCK: Block = Block::Infinite,

        fn setup(tx: mpsc::Sender<CmdFrom>, _: Option<String>) -> Web {
            let mut mount = Mount::new();
            for p in ["css", "fonts", "js"].iter() {
                mount.mount(&format!("/{}/", p),
                            Static::new(Path::new(&relpath("bootstrap")).join(p)));
            }

            let mut flows = vec! {
                Flow::new("Test flow",
                          vec! {
                              FlowState::new("One", None, vec! {
                                  FlowCmd::Message("Please insert Tab A into Slot B"),
                              }),
                          }),
                          Flow::new("Episode",
                                    vec! {
                                        // TODO shutdown button
                                        FlowState::new("Begin", None, vec! {
                                            FlowCmd::StopSensors,
                                            FlowCmd::Message("Starting new episode!"),
                                        }),
                                        // TODO generate unique ID and overall timestamp (+ timestamp for each step)

                                        FlowState::new("Camera aiming", Some(ParkState::None), vec! {
                                            FlowCmd::StopSensors,
                                            FlowCmd::Start("bluefox"), FlowCmd::Start("structure"),
                                            FlowCmd::Message("Use the Refresh button to get the cameras aimed well"),
                                        }),
                                        FlowState::new("Camera capture", None, vec! {
                                            FlowCmd::Send("bluefox disk start"),
                                            FlowCmd::Send("structure disk start"),
                                            FlowCmd::Message("Now recording! Pan the rig around to get images from various angles"),
                                        }),
                                        FlowState::new("Camera finish", None, vec! {
                                            FlowCmd::Send("bluefox disk stop"),
                                            FlowCmd::Send("structure disk stop"),
                                            FlowCmd::Message("Writing to disk, please wait..."),
                                            FlowCmd::Stop("bluefox"), FlowCmd::Stop("structure"),
                                            FlowCmd::Message("Done!"),
                                        }),

                                        FlowState::new("BioTac capture", Some(ParkState::BioTac), vec! {
                                            FlowCmd::Start("biotac"), FlowCmd::Start("stb"),
                                            FlowCmd::Message("Recording from BioTac!"),
                                        }),
                                        FlowState::new("BioTac finish", None, vec! {
                                            FlowCmd::Message("Writing to disk, please wait..."),
                                            FlowCmd::Stop("biotac"), FlowCmd::Stop("stb"),
                                            FlowCmd::Message("Done!"),
                                        }),

                                        FlowState::new("OptoForce capture", Some(ParkState::OptoForce), vec! {
                                            FlowCmd::Start("optoforce"), FlowCmd::Start("stb"),
                                            FlowCmd::Message("Recording from OptoForce!"),
                                        }),
                                        FlowState::new("OptoForce finish", None, vec! {
                                            FlowCmd::Message("Writing to disk, please wait..."),
                                            FlowCmd::Stop("optoforce"), FlowCmd::Stop("stb"),
                                            FlowCmd::Message("Done!"),
                                        }),

                                        FlowState::new("Rigid stick capture", Some(ParkState::Stick), vec! {
                                            FlowCmd::Start("stb"),
                                            FlowCmd::Message("Recording from rigid stick!"),
                                        }),
                                        FlowState::new("Rigid stick finish", None, vec! {
                                            FlowCmd::Message("Writing to disk, please wait..."),
                                            FlowCmd::Stop("stb"),
                                            FlowCmd::Message("Done!"),
                                        }),

                                        FlowState::new("Wrap up", Some(ParkState::None), vec! {
                                            FlowCmd::str("episode name"),
                                            // TODO more questions here?
                                            FlowCmd::int("hardness",     (1, 5)),
                                            FlowCmd::int("roughness",    (1, 5)),
                                            FlowCmd::int("slipperiness", (1, 5)),
                                            FlowCmd::int("warmness",     (1, 5)),
                                            FlowCmd::Message("Done!"),
                                        }),
                                    }),
            };
            let shared_flows = Arc::new(RwLock::new(flows));

            let mut router = Router::new();
            router.get("/", index(shared_flows.clone()));
            router.post("/control/:service/:action", control(tx.clone()));
            router.post("/flow/:flow/:action", flow(tx.clone(), shared_flows.clone()));

            mount.mount("/", router);

            let mut chain = Chain::new(mount);
            maybe_watch(&mut chain);
            chain.link_after(Catchall::new());

            let listening = Iron::new(chain).http(("0.0.0.0", HTTP_PORT)).unwrap();

            let (wstx, wsrx) = mpsc::channel();
            let ctx = tx.clone();
            let thread = thread::spawn(move || {

                let ws = ws::Server::bind(("0.0.0.0", WS_PORT)).unwrap();

                let ws_senders = Arc::new(Mutex::new(Vec::<ws::server::sender::Sender<_>>::new())); // FIXME why is the typechecker so confused here
                let ws_senders_clone = ws_senders.clone();
                let mut ws_relays = Vec::new();

                let marshal = thread::spawn(move || {
                    // relay messages from above to all WS threads
                    while let Ok(msg) = wsrx.recv() {
                        ws_senders_clone.lock().unwrap().iter_mut().map(|ref mut s| {
                            s.send_message(ws::Message::clone(&msg)).unwrap(); // FIXME why is the typechecker so confused here
                        }).count();
                    }

                    println!("web: shutting down websocket servers");
                    // kill all WS threads now
                    ws_senders_clone.lock().unwrap().iter_mut().map(|ref mut s| {
                        s.send_message(ws::Message::Close(None)).unwrap();
                    }).count();
                });

                for connection in ws {
                    let request = connection.unwrap().read_request().unwrap(); // Get the request
                    let headers = request.headers.clone(); // Keep the headers so we can check them
                    
                    request.validate().unwrap(); // Validate the request
                    
                    let mut response = request.accept(); // Form a response
                    
                    if let Some(&ws::header::WebSocketProtocol(ref protocols)) = headers.get() {
                        if protocols.contains(&("rust-websocket".to_string())) {
                            // We have a protocol we want to use
                            response.headers.set(ws::header::WebSocketProtocol(vec!["rust-websocket".to_string()]));
                        }
                    }
                    
                    let mut client = response.send().unwrap(); // Send the response
                    
                    let ip = client.get_mut_sender()
                        .get_mut()
                        .peer_addr()
                        .unwrap();
                    
                    println!("Websocket connection from {}", ip);
                    
                    let message = ws::Message::Text("Hello".to_string());
                    client.send_message(message).unwrap();
                    
                    let (sender, mut receiver) = client.split();
                    ws_senders.lock().unwrap().push(sender);
                    let cctx = ctx.clone();
                    ws_relays.push(thread::spawn(move || {
                        for message in receiver.incoming_messages() {
                            let message = message.unwrap();

                            match message {
                                ws::Message::Close(_) => {
                                    println!("Websocket client {} disconnected", ip);
                                    return;
                                },
                                ws::Message::Text(text) => {
                                    cctx.send(CmdFrom::Data(text)).unwrap();
                                },
                                _ => ()
                            }
                        }
                    }));
                }
            });

            Web { listening: listening, websocket: thread, wstx: wstx }
        }

        fn step(&mut self, data: Option<String>) {
            if let Some(d) = data {
                self.wstx.send(ws::Message::Text(d)).unwrap();
            }
        }
        
        fn teardown(&mut self) {
            self.listening.close().unwrap(); // FIXME this does not do anything (known bug in hyper)
            // TODO send a message to the websocket thread telling it to shut down
        }
    }
}

