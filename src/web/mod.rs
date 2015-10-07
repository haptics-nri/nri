//! Web interface to view and control running services
//!
//! Uses the Iron web framework, Handlebars templates, and Twitter Boostrap.

extern crate iron;
extern crate handlebars as hbs;
extern crate staticfile;
extern crate mount;
extern crate router;
extern crate urlencoded;
extern crate url;
extern crate hyper;
extern crate websocket as ws;
extern crate rustc_serialize as serialize;
extern crate notify;

use std::ops::Deref;
use std::path::Path;
use std::sync::{Mutex, RwLock, mpsc};
use std::thread;
use std::thread::JoinHandle;
use std::collections::{HashMap, BTreeMap};
use std::io::{self, Read, BufReader};
use std::fs::{self, File};
use super::comms::{Controllable, CmdFrom, Power, Block};
use super::teensy::ParkState;
use self::iron::prelude::*;
use self::iron::status;
use self::iron::middleware::{Handler, AfterMiddleware};
use self::iron::headers::Connection;
use self::iron::modifiers::Header;
use self::hbs::Handlebars;
use self::staticfile::Static;
use self::mount::Mount;
use self::router::Router;
#[allow(unused_imports)] use self::urlencoded::{UrlEncodedQuery, UrlEncodedBody};
use self::url::percent_encoding::percent_decode;
use self::hyper::server::Listening;
use self::hyper::header::ContentType;
use self::hyper::mime::{Mime, TopLevel, SubLevel};
use self::ws::Sender as WebsocketSender;
use self::ws::Receiver as WebsocketReceiver;
use self::serialize::json::{ToJson, Json};
use self::notify::{Watcher, RecommendedWatcher};

macro_rules! jsonize {
    ($map:ident, $selph:ident, $var:ident) => {{
        $map.insert(stringify!($var).to_owned(), $selph.$var.to_json())
    }};
    ($map:ident, $selph:ident; $($var:ident),+) => {{
        $(jsonize!($map, $selph, $var));+
    }}
}

mod flow;
mod parse;

use self::flow::Flow;

static HTTP_PORT: u16 = 3000;
static WS_PORT: u16   = 3001;

lazy_static! {
    static ref RPC_SENDERS: Mutex<HashMap<usize, mpsc::Sender<String>>>                         = Mutex::new(HashMap::new());
    static ref WS_SENDERS:  Mutex<Vec<ws::server::sender::Sender<ws::stream::WebSocketStream>>> = Mutex::new(Vec::new());
}

/// Service descriptor
///
/// Unlike the one in main.rs, this descriptor only needs to contain things that are useful for
/// display in the interface. However, they should probably be unified (TODO). The "web descriptor"
/// could be just a subfield of the main.rs service descriptor, and then those could get passed in
/// here (somehow).
struct Service {
    name      : String,
    shortname : String,
    extra     : String,
}

impl Service {
    /// Create a new service descriptor with the given name
    fn new(s: &str, t: &str, e: &str) -> Service {
        Service { name: s.to_owned(), shortname: t.to_owned(), extra: e.to_owned() }
    }
}

impl ToJson for Service {
    fn to_json(&self) -> Json {
        let mut m: BTreeMap<String, Json> = BTreeMap::new();
        jsonize!(m, self; name, shortname, extra);
        m.to_json()
    }
}

fn ws_send(wsid: usize, msg: String) {
    let mut locked_senders = WS_SENDERS.lock().unwrap();

    locked_senders[wsid].send_message(ws::Message::Text(msg)).unwrap();
}

fn ws_rpc<T, F: Fn(String) -> Result<T, String>>(wsid: usize, prompt: String, validator: F) -> T {
    let mut locked_senders = WS_SENDERS.lock().unwrap();
    let mut locked_rpcs = RPC_SENDERS.lock().unwrap();

    let mut go = |prompt: &str| -> String {
                     let (tx, rx) = mpsc::channel();
                     locked_rpcs.insert(wsid, tx);
                     locked_senders[wsid].send_message(ws::Message::Text(prompt.to_owned())).unwrap();
                     rx.recv().unwrap()
                 };

    let mut answer = go(&prompt);
    loop {
        match validator(answer) {
            Ok(ret) => return ret,
            Err(admonish) => {
                answer = go(&format!("{} {}", admonish, prompt));
            }
        }
    }
}

/// Make a path relative to the current file's directory
fn relpath(path: &str) -> String {
    String::from(Path::new(file!()).parent().unwrap().join(path).to_str().unwrap())
}

lazy_static! {
    static ref TEMPLATES: RwLock<Handlebars> = {
        const PATH: &'static str = "src/web/templates";

        fn update(hbs: &mut Handlebars) {
            let root = Path::new(PATH);
            fs::read_dir(root).unwrap()
                .take_while(Result::is_ok).map(Result::unwrap)
                .map(   |f|    f.path())
                .filter(|p|    match p.extension() { Some(ext) if ext == "hbs" => true, _ => false })
                .map(   |path| {
                    let mut source = String::from("");
                    File::open(&path).unwrap().read_to_string(&mut source).unwrap();

                    hbs.register_template_string(path.file_stem().unwrap().to_str().unwrap(), source.into()).ok().unwrap();
                }).count();
        }

        let mut hbs = Handlebars::new();
        update(&mut hbs);

        thread::spawn(move || {
            let (tx, rx) = mpsc::channel();
            let mut w: RecommendedWatcher = Watcher::new(tx).unwrap();
            w.watch(PATH).unwrap();

            for evt in rx {
                if evt.path.as_ref().unwrap().extension().unwrap() == "hbs" {
                    print!("Updating templates... ({:?} {:?})", evt.path.unwrap().file_name().unwrap(), evt.op.unwrap());
                    let mut hbs = TEMPLATES.write().unwrap();
                    update(&mut hbs);
                    println!(" done.");
                }
            }
        });

        RwLock::new(hbs)
    };

    static ref FLOWS: RwLock<Vec<Flow>> = {
        const PATH: &'static str = "src/web/flows";

        fn update(flows: &mut Vec<Flow>) {
            flows.clear();

            let root = Path::new(PATH);
            fs::read_dir(root).unwrap()
                .take_while(Result::is_ok).map(Result::unwrap)
                .map(   |f|    f.path())
                .filter(|p|    match p.extension() { Some(ext) if ext == "flow" => true, _ => false })
                .map(   |path| {
                    flows.push(parse::parse(BufReader::new(File::open(&path).unwrap())).unwrap());
                }).count();
        }

        let mut flows = vec![];
        update(&mut flows);

        thread::spawn(move || {
            let (tx, rx) = mpsc::channel();
            let mut w: RecommendedWatcher = Watcher::new(tx).unwrap();
            w.watch(PATH).unwrap();

            for evt in rx {
                if evt.path.as_ref().unwrap().extension().unwrap() == "flow" {
                    print!("Updating flows... ({:?} {:?})", evt.path.unwrap().file_name().unwrap(), evt.op.unwrap());
                    let mut flows = FLOWS.write().unwrap();
                    update(&mut flows);
                    println!(" done.");
                }
            }
        });

        RwLock::new(flows)
    };
}

/// Render a template with the data we always use
fn render(template: &str, data: BTreeMap<String, Json>) -> String {
    TEMPLATES.read().unwrap().render(template, &data).unwrap()
}

/// Handler for the main page of the web interface
fn index() -> Box<Handler> {
    Box::new(move |req: &mut Request| -> IronResult<Response> {
                      let mut data = BTreeMap::<String, Json>::new();
                      data.insert("services".to_owned(), vec![
                                  Service::new("Structure Sensor", "structure" , "<img class=\"structure latest\" /><div class=\"structure framenum\"></div>"),
                                  Service::new("mvBlueFOX3"      , "bluefox"   , "<img class=\"bluefox latest\" /><div class=\"bluefox framenum\"></div>"),
                                  Service::new("OptoForce"       , "optoforce" , ""),
                                  Service::new("SynTouch BioTac" , "biotac"    , ""),
                                  Service::new("Teensy"          , "teensy"    , ""),
                      ].to_json());
                      data.insert("flows".to_owned(), FLOWS.read().unwrap().to_json());
                      data.insert("server".to_owned(), format!("{}:{}", req.url.host, WS_PORT).to_json());

                      let mut resp = Response::new();
                      resp.set_mut(render("index", data)).set_mut(Header(ContentType(Mime(TopLevel::Text, SubLevel::Html, vec![])))).set_mut(status::Ok);
                      Ok(resp)
                  })
}

macro_rules! ignore {
    ($($t:tt)*) => {}
}

macro_rules! bind {
    ($_req:expr, $_ok:ident = $($_meth:ident).+::<$_ext:ident> () |$_p:ident, $_v:ident| $_extract:expr) => {};
    ($req:expr, $ok:ident = $($meth:ident).+::<$ext:ident> ($($var:ident),+) |$p:ident, $v:ident| $extract:expr) => {
        let ($($var,)*) = {
            if let $ok($p) = $req.$($meth).+::<$ext>() {
                ($( {
                    let $v = stringify!($var);
                    $extract
                } ,)*)
            } else {
                ($({ ignore!($var); Default::default() } ,)*)
            }
        };
    }
}

macro_rules! params {
    ($req:expr => [URL $($url:ident),*] [GET $($get:ident),*] [POST $($post:ident),*]) => {
        bind!($req, Some = extensions.get::<Router>   ($($url),*)  |p, v| String::from_utf8(percent_decode(p.find(v).unwrap().as_bytes())).unwrap());
        bind!($req, Ok   = get_ref::<UrlEncodedQuery> ($($get),*)  |p, v| p[v][0].clone());
        bind!($req, Ok   = get_ref::<UrlEncodedBody>  ($($post),*) |p, v| p[v][0].clone());
    }
}

/// Handler for controlling the NUC itself
fn nuc(tx: mpsc::Sender<CmdFrom>) -> Box<Handler> {
    let mtx = Mutex::new(tx);
    Box::new(move |req: &mut Request| -> IronResult<Response> {
                      params!(req => [URL action]
                              [GET]
                              [POST]);

                      Ok(match &*action {
                              "poweroff" => {
                                  mtx.lock().unwrap().send(CmdFrom::Power(Power::PowerOff)).unwrap();
                                  Response::with((status::Ok, "Powering off..."))
                              }
                              "reboot" => {
                                  mtx.lock().unwrap().send(CmdFrom::Power(Power::Reboot)).unwrap();
                                  Response::with((status::Ok, "Rebooting..."))
                              }
                              _ => Response::with((status::BadRequest, format!("What does {} mean?", action))),
                          })
                  })
}

/// Handler for starting/stopping a service
fn control(tx: mpsc::Sender<CmdFrom>) -> Box<Handler> {
    let mtx = Mutex::new(tx);
    Box::new(move |req: &mut Request| -> IronResult<Response> {
                      params!(req => [URL service, action]
                              [GET]
                              [POST]);

                      Ok(match &*action {
                              "start" => if rpc!(mtx.lock().unwrap(), CmdFrom::Start, service.clone()).unwrap() {
                                  Response::with((status::Ok, format!("Started {}", service)))
                              } else {
                                  Response::with((status::InternalServerError, format!("Failed to start {}", service)))
                              },
                              "stop" => if rpc!(mtx.lock().unwrap(), CmdFrom::Stop, service.clone()).unwrap() {
                                  Response::with((status::Ok, format!("Stopped {}", service)))
                              } else {
                                  Response::with((status::InternalServerError, format!("Failed to stop {}", service)))
                              },
                              "kick" => match mtx.lock().unwrap().send(CmdFrom::Data(format!("kick {}", service))) {
                                  Ok(_) => Response::with((status::Ok, format!("Kicked {}", service))),
                                  Err(_) => Response::with((status::InternalServerError, format!("Failed to kick {}", service))),
                              },
                              _ => Response::with((status::BadRequest, format!("What does {} mean?", action))),
                          })
                  })
}

/// Handler for starting/continuing a flow
fn flow(tx: mpsc::Sender<CmdFrom>) -> Box<Handler> {
    let mtx = Mutex::new(tx);
    Box::new(move |req: &mut Request| -> IronResult<Response> {
                      params!(req => [URL flow, action]
                       [GET]
                       [POST wsid]);
                      let wsid = wsid.parse().unwrap();

                      let resp = Ok(match &*action {
                              "start" | "continue" => {
                                  let mut locked_flows = FLOWS.write().unwrap();
                                  if let Some(found) = locked_flows.iter_mut().find(|f| f.name == flow) {
                                      let contour = found.run(ParkState::metermaid().unwrap(), mtx.lock().unwrap().deref(), wsid);
                                      Response::with((status::Ok, format!("{:?} \"{}\" flow", contour, flow)))
                                  } else {
                                      Response::with((status::BadRequest, format!("Could not find \"{}\" flow", flow)))
                                  }
                              }
                              _ => Response::with((status::BadRequest, format!("What does {} mean?", action))),
                          });

                      let mut data = BTreeMap::<String, Json>::new();
                      data.insert("flows".to_owned(), FLOWS.read().unwrap().to_json());
                      ws_send(wsid, String::from("flow ") + &render("flows", data));

                      resp
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
    fn new() -> Catchall {
        Catchall
    }
}

impl AfterMiddleware for Catchall {
    fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
        match err.response.status {
            Some(status::NotFound) => Ok(err.response),
            _ => Err(err),
        }
    }
}

struct Drain;

impl Drain {
    fn new() -> Drain {
        Drain
    }

    fn drain(req: &mut Request, resp: &mut Response) {
        const LIMIT: u64 = 1024 * 1024;

        io::copy(&mut req.body.by_ref().take(LIMIT), &mut io::sink()).unwrap();
        let mut buf = [0];
        if let Ok(n) = req.body.read(&mut buf) {
            if n > 0 {
                error!("Body too large, closing connection");
                resp.headers.set(Connection::close());
            }
        }
    }
}

impl AfterMiddleware for Drain {
    fn after(&self, req: &mut Request, mut resp: Response) -> IronResult<Response> {
        Drain::drain(req, &mut resp);
        Ok(resp)
    }

    fn catch(&self, req: &mut Request, mut err: IronError) -> IronResult<Response> {
        Drain::drain(req, &mut err.response);
        Err(err)
    }
}

guilty!{
    impl Controllable for Web {
        const NAME: &'static str = "web",
        const BLOCK: Block = Block::Infinite,

        fn setup(tx: mpsc::Sender<CmdFrom>, _: Option<String>) -> Web {
            let (wstx, wsrx) = mpsc::channel();
            let ctx = tx.clone();
            let thread = thread::spawn(move || {

                let ws = ws::Server::bind(("0.0.0.0", WS_PORT)).unwrap();

                let mut ws_relays = Vec::new();

                let marshal = thread::spawn(move || {
                    // relay messages from above to all WS threads
                    while let Ok(msg) = wsrx.recv() {
                        WS_SENDERS.lock().unwrap().iter_mut().map(|ref mut s| {
                            s.send_message(ws::Message::clone(&msg)).unwrap(); // FIXME why is the typechecker so confused here
                        }).count();
                    }

                    println!("web: shutting down websocket servers");
                    // kill all WS threads now
                    WS_SENDERS.lock().unwrap().iter_mut().map(|ref mut s| {
                        s.send_message(ws::Message::Close(None)).unwrap();
                    }).count();
                });

                for connection in ws {
                    let request = connection.unwrap().read_request().unwrap(); // Get the request
                    let headers = request.headers.clone(); // Keep the headers so we can check them

                    request.validate().unwrap(); // Validate the request

                    let mut response = request.accept(); // Form a response

                    if let Some(&ws::header::WebSocketProtocol(ref protocols)) = headers.get() {
                        if protocols.contains(&("rust-websocket".to_owned())) {
                            // We have a protocol we want to use
                            response.headers.set(ws::header::WebSocketProtocol(vec!["rust-websocket".to_owned()]));
                        }
                    }

                    let mut client = response.send().unwrap(); // Send the response

                    let ip = client.get_mut_sender()
                        .get_mut()
                        .peer_addr()
                        .unwrap();

                    println!("Websocket connection from {}", ip);

                    let mut locked_senders = WS_SENDERS.lock().unwrap();
                    let wsid = locked_senders.len();

                    let message = ws::Message::Text(format!("hello {}", wsid));
                    client.send_message(message).unwrap();

                    let (sender, mut receiver) = client.split();
                    locked_senders.push(sender);
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
                                    if text.starts_with("RPC") {
                                        let space = text.find(' ').unwrap();
                                        let id = text[3..space].parse::<usize>().unwrap();
                                        let msg = text[space..].to_owned();

                                        let mut locked_senders = WS_SENDERS.lock().unwrap();
                                        let locked_rpcs = RPC_SENDERS.lock().unwrap();
                                        if let Some(rpc) = locked_rpcs.get(&id) {
                                            rpc.send(msg).unwrap();
                                        } else {
                                            locked_senders[id].send_message(ws::Message::Text("RPC ERROR: nobody listening".to_owned())).unwrap();
                                        }
                                    } else {
                                        cctx.send(CmdFrom::Data(text)).unwrap();
                                    }
                                },
                                _ => ()
                            }
                        }
                    }));
                }

                marshal.join().unwrap();
            });

            let mut router = Router::new();
            router.get("/", index());
            router.post("/nuc/:action", nuc(tx.clone()));
            router.post("/control/:service/:action", control(tx.clone()));
            router.post("/flow/:flow/:action", flow(tx.clone()));

            let mut mount = Mount::new();
            for p in &["css", "fonts", "js"] {
                mount.mount(&format!("/{}/", p),
                            Static::new(Path::new(&relpath("bootstrap")).join(p)));
            }
            mount.mount("/", router);

            let mut chain = Chain::new(mount);
            chain.link_after(Catchall::new());
            chain.link_after(Drain::new());

            let listening = Iron::new(chain).http(("0.0.0.0", HTTP_PORT)).unwrap();

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
