//! Web interface to view and control running services
//!
//! Uses the Iron web framework, Handlebars templates, and Twitter Boostrap.

#[macro_use] extern crate comms;
#[macro_use] extern crate utils;
extern crate teensy;

#[macro_use] extern crate log;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate guilt_by_association;
extern crate regex;

extern crate iron;
extern crate handlebars as hbs;
extern crate staticfile;
extern crate mount;
extern crate router;
extern crate urlencoded;
extern crate url;
extern crate hyper;
extern crate rustc_serialize as serialize;
extern crate websocket;

use std::path::Path;
use std::sync::{Mutex, RwLock, mpsc};
use std::thread::JoinHandle;
use std::collections::BTreeMap;
use std::{env, str};
use std::io::Read;
use std::fs::File;
use std::process::Command;
use comms::{Controllable, CmdFrom, Power, Block};
use teensy::ParkState;
use regex::Regex;
use iron::prelude::*;
use iron::status;
use iron::middleware::Handler;
use iron::modifiers::Header;
use hbs::Handlebars;
use staticfile::Static;
use mount::Mount;
use router::Router;
#[allow(unused_imports)] use urlencoded::{UrlEncodedQuery, UrlEncodedBody};
use url::percent_encoding::percent_decode;
use hyper::server::Listening;
use hyper::header::ContentType;
use hyper::mime::{Mime, TopLevel, SubLevel};
use serialize::json::{ToJson, Json};

/// parsing and running flows
extern crate flow;
/// configuration
use utils::config;
/// a few little iron middlewares
mod middleware;
/// websocket server and utilities
mod ws;

use self::flow::{FLOWS, Comms};

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

/// Make a path relative to the current file's directory
fn relpath(path: &str) -> String {
    String::from(Path::new(file!()).parent().unwrap().parent().unwrap().join(path).to_str().unwrap())
}

lazy_static! {
    static ref TEMPLATES: RwLock<Handlebars> = utils::watch(Handlebars::new(),
                                                            &TEMPLATES,
                                                            Path::new(config::TEMPLATE_PATH),
                                                            "hbs",
                                                            |hbs, path| {
        let mut source = String::from("");
        File::open(&path).unwrap().read_to_string(&mut source).unwrap();
        hbs.register_template_string(path.file_stem().unwrap().to_str().unwrap(), source).ok().unwrap();
    });
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
                                  Service::new("Structure Sensor", "structure" , "<img class=\"frame structure latest\" /><div class=\"structure framenum\"></div>"),
                                  Service::new("mvBlueFOX3"      , "bluefox"   , "<img class=\"frame bluefox latest\" /><div class=\"bluefox framenum\"></div>"),
                                  Service::new("OptoForce"       , "optoforce" , "<img class=\"frame optoforce latest\" /><div class=\"optoforce framenum\"></div>"),
                                  Service::new("SynTouch BioTac" , "biotac"    , "<img class=\"frame biotac latest\" /><div class=\"biotac framenum\"></div>"),
                                  Service::new("Teensy"          , "teensy"    , "<img class=\"frame teensy latest\" /><div class=\"teensy framenum\"></div>"),
                                  Service::new("Vicon"           , "vicon"     , "<img class=\"frame vicon latest\" /><div class=\"vicon framenum\"></div>"),
                      ].to_json());
                      data.insert("flows".to_owned(), FLOWS.read().unwrap().to_json());
                      data.insert("server".to_owned(), format!("{}:{}", req.url.host(), config::WS_PORT).to_json());

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
        bind!($req, Some = extensions.get::<Router>   ($($url),*)  |p, v| percent_decode(p.find(v).unwrap().as_bytes()).decode_utf8_lossy().to_string());
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
                              "start" => if rpc!(mtx.lock().unwrap(), CmdFrom::Start, service.to_owned(), None).unwrap() {
                                  Response::with((status::Ok, format!("Started {}", service)))
                              } else {
                                  Response::with((status::InternalServerError, format!("Failed to start {}", service)))
                              },
                              "stop" => if rpc!(mtx.lock().unwrap(), CmdFrom::Stop, service.to_owned()).unwrap() {
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
                      let comms = ws::Comms::new(wsid);

                      let resp = Ok(match &*action {
                              a @ "start" | a @ "continue" | a @ "abort" => {
                                  let mut locked_flows = FLOWS.write().unwrap();
                                  if let Some(found) = locked_flows.get_mut(&*flow) {
                                      if a == "abort" {
                                          found.abort(&*mtx.lock().unwrap(), comms.clone()).unwrap();
                                          Response::with((status::Ok, format!("Aborting \"{}\" flow", flow)))
                                      } else {
                                          let contour = found.run(ParkState::metermaid().unwrap(), &*mtx.lock().unwrap(), comms.clone()).unwrap();
                                          Response::with((status::Ok, format!("{:?} \"{}\" flow", contour, flow)))
                                      }
                                  } else {
                                      Response::with((status::BadRequest, format!("Could not find \"{}\" flow", flow)))
                                  }
                              }
                              _ => Response::with((status::BadRequest, format!("What does {} mean?", action))),
                          });

                      let mut data = BTreeMap::<String, Json>::new();
                      data.insert("flows".to_owned(), FLOWS.read().unwrap().to_json());
                      comms.send(format!("flow {}", render("flows", data))); // FIXME if this fails, abort the flow!!! (network error)
                      comms.send(format!("diskfree {}", disk_free()));

                      resp
                  })
}

trait RegexSplit {
    fn split_re<'r, 't>(&'t self, re: &'r Regex) -> regex::Split<'r, 't>;
}

impl RegexSplit for str {
    fn split_re<'r, 't>(&'t self, re: &'r Regex) -> regex::Split<'r, 't> {
        re.split(self)
    }
}

/// Measure free disk space in gigabytes
fn disk_free() -> String {
    let re = Regex::new(r" +").unwrap();

    str::from_utf8(
        &Command::new("df") // measure disk free space
            .arg("-h")     // human-readable units
            .arg(env::current_dir().unwrap().to_str().unwrap()) // device corresponding to $PWD
            .output().unwrap().stdout).unwrap() // read all output
        .split("\n") // split lines
        .skip(1) // skip first line
        .next().unwrap() // use second line
        .split_re(&re) // split on whitespace
        .nth(3).unwrap() // fourth column is available space
        .to_owned()
}

/// Controllable struct for the web server
pub struct Web {
    /// Private handle to the HTTP server
    listening: Listening,

    /// Private handle to the websocket server thread
    websocket: Option<JoinHandle<()>>,

    /// Private channel for sending events to WebSocket clients
    wstx: Option<mpsc::Sender<ws::Message<'static>>>,
}

guilty!{
    impl Controllable for Web {
        const NAME: &'static str = "web";
        const BLOCK: Block = Block::Infinite;

        fn setup(tx: mpsc::Sender<CmdFrom>, _: Option<String>) -> Web {
            let (wstx, wsrx) = mpsc::channel();
            let ctx = tx.clone();
            let thread = ws::spawn(ctx, wsrx);

            let mut router = Router::new();
            router.get("/", index(), "index");
            router.post("/nuc/:action", nuc(tx.clone()), "nuc_action");
            router.post("/control/:service/:action", control(tx.clone()), "service_action");
            router.post("/flow/:flow/:action", flow(tx.clone()), "flow_action");

            let mut mount = Mount::new();
            for p in &["css", "fonts", "js"] {
                mount.mount(&format!("/{}/", p),
                            Static::new(Path::new(&relpath("bootstrap")).join(p)));
            }
            mount.mount("/", router);

            let mut chain = Chain::new(mount);
            chain.link_after(middleware::Catchall::new());
            chain.link_after(middleware::Drain::new());

            let listening = Iron::new(chain).http(("0.0.0.0", config::HTTP_PORT)).unwrap();

            // make sure the watchers get started
            &*FLOWS;
            &*TEMPLATES;

            Web { listening: listening, websocket: Some(thread), wstx: Some(wstx) }
        }

        fn step(&mut self, data: Option<String>) {
            if let Some(d) = data {
                self.wstx.as_ref().unwrap().send(ws::Message::text(d)).unwrap();
            }
        }

        fn teardown(&mut self) {
            self.listening.close().unwrap(); // FIXME this does not do anything (known bug in hyper)
            drop(self.wstx.take()); // (a) close the channel
            ws::ouroboros(); // (b) close the websocket listener
            self.websocket.take().unwrap().join().unwrap(); // safe to join after (a) and (b) above
        }
    }
}
