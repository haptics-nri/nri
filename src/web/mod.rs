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

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;
use std::collections::BTreeMap;
use super::comms::{Controllable, CmdFrom, Block};
use self::iron::prelude::*;
use self::iron::status;
use self::iron::middleware::Handler;
use self::hbs::{Template, HandlebarsEngine, Watchable};
use self::serialize::json::{ToJson, Json};
use self::staticfile::Static;
use self::mount::Mount;
use self::router::Router;
use self::hyper::server::Listening;
use self::ws::Sender as WebsocketSender;

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

/// Make a path relative to the current file's directory
fn relpath(path: &str) -> String {
    String::from(Path::new(file!()).parent().unwrap().join(path).to_str().unwrap())
}

/// Handler for the main page of the web interface
fn index(req: &mut Request) -> IronResult<Response> {
    let mut data = BTreeMap::<String, Json>::new();
    data.insert("services".to_string(), vec![ Service::new("Structure Sensor", "structure" , "<img class=\"structure latest\" src=\"img/structure_latest.png\" /><div class=\"structure framenum\">NaN</div>"),
                                              Service::new("mvBlueFOX3"      , "bluefox"   , "<img class=\"bluefox latest\" src=\"img/bluefox_latest.png\" /><div class=\"bluefox framenum\">NaN</div>"),
                                              Service::new("OptoForce"       ,  "optoforce", ""),
                                              Service::new("SynTouch BioTac" ,  "biotac"   , ""),
                                              Service::new("STB"             ,  "stb"      , ""),
                                            ].to_json());
    data.insert("server".to_string(), format!("{}:{}", req.url.host, WS_PORT).to_json());

    let mut resp = Response::new();
    resp.set_mut(Template::new("index", data)).set_mut(status::Ok);
    Ok(resp)
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

/// Controllable struct for the web server
pub struct Web {
    /// Private handle to the HTTP server
    listening: Listening,

    /// Private handle to the websocket server thread
    websocket: JoinHandle<()>,

    /// Private channel for sending events to WebSocket clients
    wstx: mpsc::Sender<ws::Message>,
}

guilty!{
    impl Controllable for Web {
        const NAME: &'static str = "web",

        fn setup(tx: mpsc::Sender<CmdFrom>, _: Option<String>) -> Web {
            let mut mount = Mount::new();
            for p in ["css", "fonts", "js"].iter() {
                mount.mount(&format!("/{}/", p),
                            Static::new(Path::new(&relpath("bootstrap")).join(p)));
            }

            let mut router = Router::new();
            router.get("/", index);
            router.post("/control/:service/:action", control(tx));

            mount.mount("/", router);

            let mut chain = Chain::new(mount);

            let watcher = Arc::new(HandlebarsEngine::new(&relpath("templates"), ".hbs"));
            watcher.watch();

            chain.link_after(watcher);

            let listening = Iron::new(chain).http(("0.0.0.0", HTTP_PORT)).unwrap();

            let (wstx, wsrx) = mpsc::channel();
            let thread = thread::spawn(move || {

                let ws = ws::Server::bind(("0.0.0.0", WS_PORT)).unwrap();

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
                    
                    let (mut sender, mut receiver) = client.split();
                    // TODO multiplex the receiver with the mpsc Receiver somehow
                    
                    /*for message in receiver.incoming_messages() {
                        let message = message.unwrap();
                        
                        match message {
                            ws::Message::Close(_) => {
                                let message = ws::Message::Close(None);
                                sender.send_message(message).unwrap();
                                println!("Websocket client {} disconnected", ip);
                                return;
                            }
                            ws::Message::Ping(data) => {
                                let message = ws::Message::Pong(data);
                                sender.send_message(message).unwrap();
                            }
                            _ => sender.send_message(message).unwrap(),
                        }
                    }*/
                    while let Ok(msg) = wsrx.recv() {
                        sender.send_message(msg).unwrap();
                    }
                }
            });

            Web { listening: listening, websocket: thread, wstx: wstx }
        }

        fn step(&mut self, data: Option<String>) -> Block {
            if let Some(d) = data {
                self.wstx.send(ws::Message::Text(d)).unwrap();
            }

            Block::Infinite
        }
        
        fn teardown(&mut self) {
            self.listening.close().unwrap(); // FIXME this does not do anything (known bug in hyper)
            // TODO send a message to the websocket thread telling it to shut down
        }
    }
}

