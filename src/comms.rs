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

/// Convenience macro for making an "RPC" call from a service up to the main thread. This is done
/// by generating a nonce channel, stuffing the sending end into a message that gets sent to the
/// main thread, and then waiting for the main thread to send back a reply.
/// 
/// This is a macro instead of a function because you need to pass in the _name_ of a CmdFrom
/// variant without constructing it (because a Sender is needed to construct it, but the macro is
/// creating the channel for you). Possibly a better design would be structs and a trait/generic.
///
/// * Inputs:
///     - tx: Sender\<CmdFrom> = Sender that the service uses to send commands to the main thread
///     - name = name of a CmdFrom variant that has a Sender<T> as the last parameter
///     - params = any other parameters for the CmdFrom variant
/// * Outputs:
///     - Result\<T, mpsc::RecvError> = the response received (or not) from the main thread
#[macro_export]
macro_rules! rpc {
    ($tx:expr, CmdFrom::$name:ident, $($param:expr),*) => {{
        let (msg_tx, msg_rx) = ::std::sync::mpsc::channel();
        $tx.send($crate::comms::CmdFrom::$name($($param),*, msg_tx));
        msg_rx.recv()
    }}
}

/// Convenience macro for defining a stub service that doesn't do anything (yet). Defines a
/// zero-sized struct and an impl that blocks between receiving messages from the main thread (so
/// it doesn't do anything, but it doesn't sit in a CPU-busy loop either).
///
/// * Inputs:
///     - t = name of the Controllable
/// * Items created:
///     - pub struct (named $t) and stub impl
macro_rules! stub {
    ($t:ident) => {
        pub struct $t;
        impl $crate::comms::Controllable for $t {
            fn setup(tx: ::std::sync::mpsc::Sender<$crate::comms::CmdFrom>) -> $t {
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

