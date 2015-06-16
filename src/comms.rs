//! Utilities for communication between the supervisor thread and services

use std::sync::mpsc::{channel, Sender, Receiver, RecvError, TryRecvError};

/// Commands sent from the supervisor thread to services
#[derive(Clone)]
pub enum CmdTo {
    /// Start the service
    Start,

    /// Stop the service (but keep the thread running)
    Stop,

    /// Stop the service and kill the thread
    Quit
}

/// Commands sent from services up to the supervisor thread
#[derive(Clone)]
pub enum CmdFrom {
    /// Start another service
    Start(String, Sender<bool>),

    /// Stop another service
    Stop(String, Sender<bool>),

    /// Shut down everything
    Quit
}

/// A service that can be setup and torn down based on commands from a higher power.
pub trait Controllable {
    /// Setup the service.
    ///
    /// Should initialize any necessary libraries and devices. May be called more than once, but
    /// teardown() will be called in between.
    fn setup(Sender<CmdFrom>) -> Self;

    /// Run one "step".
    ///
    /// In the case of a device driver, this corresponds to gathering one frame or sample of data.
    ///
    /// Return true if we should wait for a command from the supervisor thread before calling
    /// step() again. Return false to call step() again right away (unless there is a pending
    /// command).
    fn step(&mut self) -> bool;

    /// Tear down the service.
    ///
    /// Should shut down any necessary libraries or services. Either the program is exiting, or the
    /// service is just being paused (setup() could be called again).
    fn teardown(&mut self);
}

// (Sender<CmdFrom>, name of CmdFrom variant; CmdFrom variant params) -> Result<reply, RecvError>
macro_rules! rpc {
    ($tx:expr, CmdFrom::$name:ident; $($param:expr),*) => {{
        let (msg_tx, msg_rx) = channel();
        $tx.send(CmdFrom::$name($($param),*, msg_tx));
        msg_rx.recv()
    }}
}

macro_rules! stub {
    ($t:ident) => {
        pub struct $t;
        impl Controllable for $t {
            fn setup(tx: Sender<CmdFrom>) -> $t {
                $t
            }

            fn step(&mut self) -> bool {
                true
            }

            fn teardown(&mut self) {
            }
        }
    }
}

/// Service driving function
///
/// Runs in a loop receiving commands from th supervisor thread. Manages a Controllable instance,
/// calling its setup()/step()/teardown() methods as necessary.
pub fn go<C: Controllable>(rx: Receiver<CmdTo>, tx: Sender<CmdFrom>) {
    loop {
        match rx.recv() {
            Ok(cmd) => match cmd {
                CmdTo::Start => {}, // let's go!
                CmdTo::Stop | CmdTo::Quit => return, // didn't even get to start
            },
            Err(e) => return, // main thread exploded?
        }

        let mut c = C::setup(tx.clone());
        let mut should_block = false;

        loop {
            if should_block {
                match rx.recv() {
                    Ok(cmd) => match cmd {
                        CmdTo::Start => {}, // already started
                        CmdTo::Stop => break, // shutdown command
                        CmdTo::Quit => { c.teardown(); return }, // real shutdown command
                    },
                    Err(_) => { c.teardown(); return }
                }
            } else {
                match rx.try_recv() {
                    Ok(cmd) => match cmd {
                        CmdTo::Start => {}, // already started
                        CmdTo::Stop => break, // shutdown command
                        CmdTo::Quit => { c.teardown(); return }, // real shutdown command
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

