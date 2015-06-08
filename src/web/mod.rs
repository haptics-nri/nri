extern crate iron;
extern crate handlebars_iron as hbs;
extern crate staticfile;
extern crate mount;
extern crate hyper;

use std::path::Path;
use super::comms::Controllable;
use self::iron::prelude::*;
use self::iron::status;
use self::hbs::{Template, HandlebarsEngine};
use self::staticfile::Static;
use self::mount::Mount;
use self::hyper::server::Listening;

fn relpath(path: &str) -> String {
    String::from(Path::new(file!()).parent().unwrap().join(path).to_str().unwrap())
}

fn index(req: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    resp.set_mut(Template::new("index", {})).set_mut(status::Ok);
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

        chain.link_after(HandlebarsEngine::new(&relpath("templates"), ".hbs"));

        let listening = Iron::new(chain).http("0.0.0.0:3000").unwrap();

        Web { listening: listening }
    }

    fn step(&mut self) {
    }
    
    fn teardown(&mut self) {
        self.listening.close().unwrap(); // FIXME this does not do anything (known bug in hyper)
    }
}

