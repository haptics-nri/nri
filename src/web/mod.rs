extern crate iron;
extern crate handlebars_iron as hbs;

use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError};
use super::comms::Cmd;
use self::iron::prelude::*;
use self::iron::status;
use self::hbs::{Template, HandlebarsEngine};

pub fn go(rx: Receiver<Cmd>) {
    match rx.recv() {
        Ok(cmd) => match cmd {
            Cmd::Start => {}, // let's go!
            Cmd::Stop => return, // didn't even get to start
        },
        Err(e) => return, // main thread exploded?
    }

    let mut chain = Chain::new(|_: &mut Request| {
        let mut resp = Response::new();
        resp.set_mut(Template::new("index", {})).set_mut(status::Ok);
        Ok(resp)
    });

    chain.link_after(HandlebarsEngine::new(Path::new(file!()).parent().unwrap().join("templates").to_str().unwrap(), ".hbs"));

    let mut listening = Iron::new(chain).http("localhost:3000").unwrap();

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

