extern crate iron;
extern crate handlebars_iron as hbs;
extern crate staticfile;
extern crate mount;
extern crate hyper;
extern crate rustc_serialize as serialize;

use std::path::Path;
use std::sync::Arc;
use std::collections::BTreeMap;
use super::comms::Controllable;
use self::iron::prelude::*;
use self::iron::status;
use self::hbs::{Template, HandlebarsEngine, Watchable};
use self::serialize::json::{ToJson, Json};
use self::staticfile::Static;
use self::mount::Mount;
use self::hyper::server::Listening;

struct Service {
    name: String,
}

impl Service {
    fn new(s: &str) -> Service {
        Service { name: s.to_string() }
    }
}

impl ToJson for Service {
    fn to_json(&self) -> Json {
        let mut m: BTreeMap<String, Json> = BTreeMap::new();
        m.insert("name".to_string(), self.name.to_json());
        m.to_json()
    }
}

fn relpath(path: &str) -> String {
    String::from(Path::new(file!()).parent().unwrap().join(path).to_str().unwrap())
}

fn index(req: &mut Request) -> IronResult<Response> {
    let mut data = BTreeMap::<String, Json>::new();
    data.insert("services".to_string(), vec![ Service::new("Structure Sensor"),
                                              Service::new("mvBlueFOX3"),
                                              Service::new("OptoForce"),
                                              Service::new("SynTouch BioTac"),
                                            ].to_json());

    let mut resp = Response::new();
    resp.set_mut(Template::new("index", data)).set_mut(status::Ok);
    Ok(resp)
}

pub struct Web {
    listening: Listening,
}

impl Controllable<Web> for Web {
    fn setup() -> Web {
        let mut mount = Mount::new();
        for p in ["css", "fonts", "js"].iter() {
            mount.mount(&format!("/{}/", p),
                        Static::new(Path::new(&relpath("bootstrap")).join(p)));
        }

        mount.mount("/", index);

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

