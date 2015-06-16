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

use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};
use std::collections::BTreeMap;
use super::comms::{Controllable, CmdFrom};
use self::iron::prelude::*;
use self::iron::{status, typemap};
use self::iron::middleware::Handler;
use self::hbs::{Template, HandlebarsEngine, Watchable};
use self::serialize::json::{ToJson, Json};
use self::staticfile::Static;
use self::mount::Mount;
use self::router::Router;
use self::hyper::server::Listening;

/// Service descriptor
///
/// Unlike the one in main.rs, this descriptor only needs to contain things that are useful for
/// display in the interface. However, they should probably be unified (TODO). The "web descriptor"
/// could be just a subfield of the main.rs service descriptor, and then those could get passed in
/// here (somehow).
struct Service {
    name: String,
    shortname: String,
}

impl Service {
    /// Create a new service descriptor with the given name
    fn new(s: &str, t: &str) -> Service {
        Service { name: s.to_string(), shortname: t.to_string() }
    }
}

impl ToJson for Service {
    fn to_json(&self) -> Json {
        let mut m: BTreeMap<String, Json> = BTreeMap::new();
        m.insert("name".to_string(), self.name.to_json());
        m.insert("shortname".to_string(), self.shortname.to_json());
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
    data.insert("services".to_string(), vec![ Service::new("Structure Sensor", "structure"),
                                              Service::new("mvBlueFOX3",       "bluefox"  ),
                                              Service::new("OptoForce",        "optoforce"),
                                              Service::new("SynTouch BioTac",  "biotac"   ),
                                            ].to_json());

    let mut resp = Response::new();
    resp.set_mut(Template::new("index", data)).set_mut(status::Ok);
    Ok(resp)
}

/// Handler for starting/stopping a service
fn control(tx: Sender<CmdFrom>) -> Box<Handler> {
    let mtx = Mutex::new(tx);
    Box::new(move |req: &mut Request| -> IronResult<Response> {
        let params = req.extensions.get::<Router>().unwrap();
        let service = params.find("service").unwrap();
        let action = params.find("action").unwrap();

        match action {
            "start" =>
                if rpc!(mtx.lock().unwrap(), CmdFrom::Start; service.to_string()).unwrap() {
                    Ok(Response::with((status::Ok, format!("Started {}", service))))
                } else {
                    Ok(Response::with((status::InternalServerError, format!("Failed to start {}", service))))
                },
            "stop" =>
                if rpc!(mtx.lock().unwrap(), CmdFrom::Stop; service.to_string()).unwrap() {
                    Ok(Response::with((status::Ok, format!("Stopped {}", service))))
                } else {
                    Ok(Response::with((status::InternalServerError, format!("Failed to stop {}", service))))
                },
            _ => Ok(Response::with((status::BadRequest, format!("What does {} mean?", action))))
        }
    })
}

/// Controllable struct for the web server
pub struct Web {
    /// Private handle to the server
    listening: Listening,
}

impl Controllable for Web {
    fn setup(tx: Sender<CmdFrom>) -> Web {
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

        let listening = Iron::new(chain).http("0.0.0.0:3000").unwrap();

        Web { listening: listening }
    }

    fn step(&mut self) -> bool {
        true
    }
    
    fn teardown(&mut self) {
        self.listening.close().unwrap(); // FIXME this does not do anything (known bug in hyper)
    }
}

