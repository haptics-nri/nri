extern crate iron;

use std::sync::mpsc::{Receiver, TryRecvError};
use super::comms::Cmd;

pub fn go(rx: Receiver<Cmd>) {
    match rx.recv() {
        Ok(cmd) => match cmd {
            Cmd::Start => {}, // let's go!
            Cmd::Stop => return, // didn't even get to start
        },
        Err(e) => return, // main thread exploded?
    }

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
}

