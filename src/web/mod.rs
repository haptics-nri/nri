extern crate iron;
extern crate handlebars_iron as hbs;
extern crate staticfile;
extern crate mount;

use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError};
use super::comms::Cmd;
use self::iron::prelude::*;
use self::iron::status;
use self::hbs::{Template, HandlebarsEngine};
use self::staticfile::Static;
use self::mount::Mount;

fn relpath(path: &str) -> String {
    String::from(Path::new(file!()).parent().unwrap().join(path).to_str().unwrap())
}

macro_rules! as_str { ($s:expr) => ( &$s[..] ) }
macro_rules! relpath { ($s:expr) => ( as_str!(relpath($s)) ) }

fn index(req: &mut Request) -> IronResult<Response> {
    let mut resp = Response::new();
    resp.set_mut(Template::new("index", {})).set_mut(status::Ok);
    Ok(resp)
}

pub fn go(rx: Receiver<Cmd>) {
    match rx.recv() {
        Ok(cmd) => match cmd {
            Cmd::Start => {}, // let's go!
            Cmd::Stop => return, // didn't even get to start
        },
        Err(e) => return, // main thread exploded?
    }

    let mut mount = Mount::new();
    for p in ["css", "fonts", "js"].iter() {
        mount.mount(as_str!(format!("/{}/", p)),
                    Static::new(Path::new(relpath!("bootstrap")).join(p)));
    }

    mount.mount("/", index);

    let mut chain = Chain::new(mount);

    chain.link_after(HandlebarsEngine::new(relpath!("templates"), ".hbs"));

    let mut listening = Iron::new(chain).http("0.0.0.0:3000").unwrap();

    loop {
        match rx.try_recv() {
            Ok(cmd) => match cmd {
                Cmd::Start => {}, // already started
                Cmd::Stop => break, // shutdown command
            },
            Err(e) => match e {
                TryRecvError::Empty => {}, // continue
                TryRecvError::Disconnected => break, // main thread exploded?
            },
        }
    }

    listening.close().unwrap(); // FIXME this does not do anything (known bug in hyper)
}

