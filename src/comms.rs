//! Utilities for communication between the supervisor thread and services

extern crate time;
extern crate libc;

use std::sync::mpsc::{channel, Sender, Receiver, TryRecvError, SendError};
use std::thread;
use std::{mem, ptr};
use super::hprof;
use self::libc::{nanosleep, timespec};

/// Commands sent from the supervisor thread to services
#[derive(Clone)]
pub enum CmdTo {
    /// Start the service
    Start,

    /// Stop the service (but keep the thread running)
    Stop,

    /// Stop the service and kill the thread
    Quit,

    /// Some data
    // TODO enum instead of str, somehow each service defines its own variants
    Data(String),
}

/// Commands sent from services up to the supervisor thread
#[derive(Clone)]
pub enum CmdFrom {
    /// Start another service
    Start(String, Sender<bool>),

    /// Stop another service
    Stop(String, Sender<bool>),

    /// Some data
    // TODO enum instead of str, somehow each service defines its own variants
    Data(String),

    /// Shut down everything
    Quit,

    /// Abort the main thread immediately (never do this)
    Panic,

    /// Schedule the sending thread to be killed in x ms
    Timeout(&'static str, u32),
    /// Cancel a killing scheduled with Timeout
    Timein(&'static str),
}

/// Desired blocking mode for a service
///
/// Defines what the go() function does in between calls to Controllable::step()
#[derive(Debug, Copy, Clone)]
pub enum Block {
    /// Do not call step() again until there is a command from on high. Used by services that do
    /// all the work in background threads or have nothing to do.
    Infinite,
    
    /// No delay -- call step() again immediately. Used by IO-bound or CPU-bound services.
    Immediate,

    /// Request a period N (in nanoseconds) managed by go(). Used by services that are not IO-bound
    /// or CPU-bound.  Assuming each call to step() completes in less than N ns, step() will be
    /// called approximately every N ns. A simple proportional controller is used to adjust the
    /// sleep time, to account for unmeasured delays and get as close as possible to the desired
    /// period.
    Period(i64)
}

guilty!{
    /// A service that can be setup and torn down based on commands from a higher power.
    pub trait Controllable {
        // FIXME no docs allowed yet on guilty consts
        // Human-readable name of the service
        const NAME: &'static str,
        // Desired blocking mode (see documentation for the Block enum)
        const BLOCK: Block,

        /// Setup the service.
        ///
        /// Should initialize any necessary libraries and devices. May be called more than once, but
        /// teardown() will be called in between.
        fn setup(Sender<CmdFrom>, Option<String>) -> Self;

        /// Run one "step".
        ///
        /// In the case of a device driver, this corresponds to gathering one frame or sample of data.
        ///
        /// Return true if we should wait for a command from the supervisor thread before calling
        /// step() again. Return false to call step() again right away (unless there is a pending
        /// command).
        fn step(&mut self, data: Option<String>);

        /// Tear down the service.
        ///
        /// Should shut down any necessary libraries or services. Either the program is exiting, or the
        /// service is just being paused (setup() could be called again).
        fn teardown(&mut self);
    }
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
        $tx.send($crate::comms::CmdFrom::$name($($param),*, msg_tx)).unwrap();
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
#[macro_export]
macro_rules! stub {
    ($t:ident) => {
        pub struct $t;

        use super::comms::{Controllable, Block, CmdFrom};
        guilty!{
            impl Controllable for $t {
                const NAME: &'static str = concat!("Stub ", stringify!($t)),
                const BLOCK: Block = Block::Infinite,

                fn setup(_: ::std::sync::mpsc::Sender<CmdFrom>, _: Option<String>) -> $t {
                    $t
                }

                fn step(&mut self, _: Option<String>) {
                }

                fn teardown(&mut self) {
                }
            }
        }
    }
}

/// Represents the two loops in go(), used in conjunction with maybe_break!
enum Break { Running, Alive }

// TODO move into utility macros mod
/// Pass-through macro for re-interpreting a token tree as an expression
#[macro_export]
macro_rules! as_expr { ($e:expr) => ($e) }

/// Helper macro for go()
///
/// Breaks out of the run loop, the life loop, or does nothing based on return value from handle()
#[macro_export]
macro_rules! maybe_break {
    ($v:expr, $running:tt, $alive:tt) => (as_expr!({
        match $v {
            Some(Break::Running) => break $running,
            Some(Break::Alive)   => break $alive,
            None => ()
        }
    }))
}

/// Helper function for handle()
///
/// Called in the case of a command from the main thread when the service is already running
fn handle_ok<C: Controllable>(cmd: CmdTo, c: &mut C, data: &mut Option<String>) -> Option<Break> {
    match cmd {
        CmdTo::Start => {}, // already started
        CmdTo::Stop => return Some(Break::Running), // shutdown command
        CmdTo::Quit => return handle_err(c), // real shutdown command
        CmdTo::Data(d) => *data = Some(d), // have data!
    }
    None
}

/// Helper function for handle()
///
/// Called in the case of a communication error or shutdown command from the main thread
fn handle_err<C: Controllable>(c: &mut C) -> Option<Break> {
    c.teardown(); // FIXME is this necessary or should I just impl Drop
    Some(Break::Alive)
}

/// Helper function for go()
///
/// Handles communications from the main thread
fn handle<C: Controllable>(block: bool, c: &mut C, rx: &Receiver<CmdTo>, data: &mut Option<String>) -> Option<Break> {
    if block {
        match rx.recv() {
            Ok(cmd) => handle_ok(cmd, c, data),
            Err(_) => handle_err(c)
        }
    } else {
        match rx.try_recv() {
            Ok(cmd) => handle_ok(cmd, c, data),
            Err(e) => match e {
                TryRecvError::Empty => None, // continue
                TryRecvError::Disconnected => handle_err(c), // main thread exploded?
            },
        }
    }
}

/// Service driving function
///
/// Runs in a loop receiving commands from the supervisor thread. Manages a Controllable instance,
/// calling its setup()/step()/teardown() methods as necessary.
pub fn go<C: Controllable>(rx: Receiver<CmdTo>, tx: Sender<CmdFrom>) {
    'alive: loop {
        let mut data = None;

        'hatching: loop {
            match rx.recv() {
                Ok(cmd) => match cmd {
                    CmdTo::Start => break 'hatching, // let's go!
                    CmdTo::Data(_) => continue 'hatching, // sorry, not listening yet
                    CmdTo::Stop | CmdTo::Quit => break 'alive, // didn't even get to start
                },
                Err(_) => return, // main thread exploded?
            }
        }

        tx.send(CmdFrom::Timeout(guilty!(C::NAME), 1000)).unwrap();
        let mut c = C::setup(tx.clone(), data);
        tx.send(CmdFrom::Timein(guilty!(C::NAME))).unwrap();

        let mut block = guilty!(C::BLOCK);
        let actual_period = match block {
            Block::Immediate => 0,
            Block::Infinite  => -1,
            Block::Period(period) => period
        };

        super::PROF.with(|wrapped_prof| {
            *wrapped_prof.borrow_mut() = Some(hprof::Profiler::new(guilty!(C::NAME)));
        });

        let mut i = 0;
        let mut life_start = time::now();
        'running: loop {
            data = None;
            i += 1;

            let start = time::now();

            match block {
                Block::Immediate => {
                    maybe_break!(handle(false, &mut c, &rx, &mut data), 'running, 'alive);
                    prof!("step", c.step(data));
                },
                Block::Infinite  => {
                    maybe_break!(handle(true, &mut c, &rx, &mut data), 'running, 'alive);
                    prof!("step", c.step(data));
                }
                Block::Period(desired_period) => {
                    maybe_break!(handle(false, &mut c, &rx, &mut data), 'running, 'alive);
                    prof!("step", c.step(data));
                    if let Some(nanos) = (time::now() - start).num_nanoseconds() {
                        if nanos < desired_period {
                            unsafe { nanosleep(&timespec { tv_sec: 0, tv_nsec: desired_period - nanos }, ptr::null_mut()); }
                        }
                    }

                    if i == 100 {
                        if let Some(nanos) = (time::now() - life_start).num_nanoseconds() {
                            let duration = nanos as f64 / i as f64;
                            block = Block::Period(desired_period + actual_period - duration as i64);
                        }
                        i = 0;
                        life_start = time::now();
                    }
                }

            }
        }

        c.teardown();
    }

    println!("\n\n");
    super::PROF.with(|wrapped_prof| {
        if let Some(ref prof) = *wrapped_prof.borrow() {
            prof.print_timing();
        }
    });
}

/// Container for a thread that repeatedly performs some action in response to input. Can be
/// stopped and restarted.
pub struct RestartableThread<Data: Send + 'static> {
    /// Sending end of a channel used to send inputs to the thread.
    ///
    /// Wrapped in a Option so it can be dropped without moving self.
    tx: Option<Sender<Data>>,

    /// Handle to the running thread
    ///
    /// Wrapped in an option so it can be joined without moving self.
    thread: Option<thread::JoinHandle<()>>
}

impl<Data: Send + 'static> RestartableThread<Data> {
    /// Create a new RestartableThread which performs the given action in response to input.
    /// The thread will run (and wait for input) until RestartableThread::join() is called or the
    /// RestartableThread instance is dropped.
    /// To pass input, use RestartableThread::send().
    pub fn new<F>(n: &'static str, f: F) -> RestartableThread<Data> where F: Send + 'static + Fn(Data)
    {
        let (tx, rx) = channel();
        RestartableThread {
            tx: Some(tx),
            thread: Some(thread::spawn(move || {
                super::PROF.with(|wrapped_prof| {
                    *wrapped_prof.borrow_mut() = Some(hprof::Profiler::new(n));
                });

                while let Ok(x) = rx.recv() {
                    prof!("step", f(x));
                }

                super::PROF.with(|wrapped_prof| {
                    if let Some(ref prof) = *wrapped_prof.borrow() {
                        prof.print_timing();
                    }
                });
            }))
        }
    }

    /// Kill the thread. This shuts down the message queue, causing the thread to exit, and then
    /// waits for it to finish up any outstanding work. No deadlocks here!
    pub fn join(&mut self) {
        if self.thread.is_some() {
            self.tx = None; // this causes the Sender to be dropped

            let mut old_thread = None;
            mem::swap(&mut old_thread, &mut self.thread);
            old_thread.unwrap().join().unwrap(); // safe to join since we hung up the channel
        }
    }

    /// Send some input to the thread. Nonblocking.
    /// Returns a SendError if Sender::send() fails or if the private Sender has somehow
    /// disappeared (which is impossible).
    pub fn send(&self, d: Data) -> Result<(), SendError<Data>> {
        if let Some(ref s) = self.tx {
            s.send(d)
        } else {
            Err(SendError(d))
        }
    }
}

impl<Data: Send + 'static> Drop for RestartableThread<Data> {
    /// When the RestartableThread goes out of scope, kill the thread.
    fn drop(&mut self) {
        self.join();
    }
}

