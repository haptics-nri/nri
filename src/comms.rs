//! Utilities for communication between the supervisor thread and services

use std::sync::mpsc::{Receiver, TryRecvError};

#[derive(Clone)]
pub enum Cmd {
    Start,
    Stop ,
    Quit ,
}

/// A service that can be setup and torn down based on commands from a higher power.
pub trait Controllable<C> {
    fn setup() -> C;
    fn step(&mut self) -> bool;
    fn teardown(&mut self);
}

pub fn go<C: Controllable<C>>(rx: Receiver<Cmd>) {
    loop {
        match rx.recv() {
            Ok(cmd) => match cmd {
                Cmd::Start => {}, // let's go!
                Cmd::Stop | Cmd::Quit => return, // didn't even get to start
            },
            Err(e) => return, // main thread exploded?
        }

        let mut c = C::setup();
        let mut should_block = false;

        loop {
            if should_block {
                match rx.recv() {
                    Ok(cmd) => match cmd {
                        Cmd::Start => {}, // already started
                        Cmd::Stop => break, // shutdown command
                        Cmd::Quit => { c.teardown(); return }, // real shutdown command
                    },
                    Err(_) => { c.teardown(); return }
                }
            } else {
                match rx.try_recv() {
                    Ok(cmd) => match cmd {
                        Cmd::Start => {}, // already started
                        Cmd::Stop => break, // shutdown command
                        Cmd::Quit => { c.teardown(); return }, // real shutdown command
                    },
                    Err(e) => match e {
                        TryRecvError::Empty => {}, // continue
                        TryRecvError::Disconnected => { c.teardown(); return }, // main thread exploded?
                    },
                }
            }

            should_block = c.step();
        }

        c.teardown();
    }
}

