//! This crate contains all the software used to run the sensing rig. It's designed to run on the
//! Intel NUC in the rig operator's backpack. There are driver wrappers for each sensor, a
//! supervisor that runs each of those in its own thread, plus CLI and web interfaces to control
//! everything.
//!
//! # How to...
//!
//! ## ... generate this documentation
//!
//! - Run cargo doc in the NRI repo:
//!
//! <pre>nri$ cargo doc
//! </pre>
//! - Now we need the rustdoc command for this crate.
//!
//! <pre>
//! nri$ touch src/main.rs
//! nri$ cargo doc -v | grep rustdoc | awk "-FRunning " '{print substr($NF, 2, length($NF)-2)}' > cargo-doc-command
//! </pre>
//! - Edit <code>cargo-doc-command</code> and add <code>--no-defaults --passes "collapse-docs" --passes "unindent-comments"</code> after <code>src/main.rs</code>. Then run it.
//!
//! <pre>
//! nri$ source cargo-doc-command
//! </pre>
//! - You can now access the docs at <code>target/doc/nri/index.html</code>. If you want them on the web: copy the docs to the Github Pages repo, commit, and push.
//!
//! <pre>
//! nri$ pushd ../haptics-nri.github.io
//! haptics-nri.github.io$ git pull
//! haptics-nri.github.io$ popd
//! nri$ rsync -a target/doc ../haptics-nri.github.io
//! nri$ pushd ../haptics-nri.github.io
//! haptics-nri.github.io$ git add doc
//! haptics-nri.github.io$ git commit -m "cargo doc"
//! haptics-nri.github.io$ git push
//! haptics-nri.github.io$ popd
//! </pre>
//! - The docs are now live (after 30s or so) at http://haptics-nri.github.io/doc/nri.
//!
//! ## ... lint this code
//!
//! - The script <code>clippy.sh</code> modifies Cargo.toml and src/main.rs to include [rust-clippy](https://crates.io/crates/clippy), changes the toolchain to nightly (using [multirust](https://github.com/brson/multirust)), runs <code>cargo run</code> to generate all the lint warnings, and then switches everything back.
//!
//! ## ... set up the wi-fi hotspot
//!
//! I followed the instructions [here](http://ubuntuhandbook.org/index.php/2014/09/3-ways-create-wifi-hotspot-ubuntu/) to create a Wi-Fi hotspot to which Android devices can connect. Unity's built in network manager can almost, but not quite, do it. You need to create the network in the manager and then go edit the file to change it from Infrastructure Mode to AP Mode (which is not an option in the GUI -- you can select Ad-hoc Mode, but Android won't connect to that).
//!
//! Shortened instructions:
//!
//! 1. Click on the network icon in the system tray. Select "Edit connections...".
//! 2. Click "Add".
//! 3. Choose "Wi-Fi" for the type and click Create.
//! 4. Make up an SSID. Leave the type as "Infrastructure" (it doesn't matter, since we'll change
//!    it manually). Select the wireless card in the "Device MAC address" dropdown.
//! 5. Go to the "Wi-Fi Security" tab and choose sane options.
//! 6. Go to the "IPv4 Settings" tab and set the Method to "Shared to other computers".
//! 7. Click Save.
//! 8. Edit the file <code>/etc/NetworkManager/system-connections/$SSID</code> and change
//!    <code>mode=infrastructure</code> to <code>mode=ap</code>.
//! 9. Deactivate and reactivate Wi-Fi. Then you should be able to select the new SSID to
//!    "connect" (meaning broadcast). And then you should be able to connect from other devices!
//!
//! Note that (obviously) when the NUC is running as a hotspot, it has no internet connection. I
//! tried to get DNS running, but I failed, so for now you have to use an IP address to access the
//! NUC. You can see what it chose by running <code>hostname -I</code> -- it seems to like 10.42.0.1.

#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

/// Just like println!, but prints to stderr
#[macro_export]
macro_rules! errorln {
    ($($arg:tt)*) => {{
        use std::io::Write;
        match writeln!(&mut ::std::io::stderr(), $($arg)* ) {
            Ok(_) => {},
            Err(x) => panic!("Unable to write to stderr: {}", x),
        }
    }}
}

#[macro_export]
macro_rules! group_attr {
    (#[cfg($attr:meta)] $($yes:item)*) => { group_attr!{internal #[cfg($attr)] $($yes)* } };

    ($modname:ident #[cfg($attr:meta)] $($yes:item)*) => {
        #[cfg($attr)]
        mod $modname {
            $($yes)*
        }

        #[cfg(not($attr))]
        mod $modname {
        }

        pub use self::$modname::*;
    };
}

#[macro_use]
extern crate guilt_by_association;

// TODO move this profiling stuff to a mod
use std::cell::RefCell;

thread_local! {
    // Thread-local profiler object (FIXME no doc comments on thread locals)
    static PROF: RefCell<Option<hprof::Profiler>> = RefCell::new(None)
}

#[macro_export]
macro_rules! prof {
    ($b:expr) => { prof!(stringify!($b), $b) };
    //($n:expr, $b:expr) => ($b)
    ($n:expr, $b:expr) => {{
        $crate::PROF.with(|wrapped_prof| {
            let appease_borrowck = wrapped_prof.borrow();
            let g = match *appease_borrowck {
                Some(ref prof) => prof.enter($n),
                None => $crate::hprof::enter($n)
            };
            let ret = { $b }; //~ ALLOW let_unit_value
            drop(g);
            ret
        })
    }}
}

#[macro_use] mod comms;
mod scribe;
mod cli;
mod web;
mod stb;
mod optoforce;
mod structure;
mod bluefox;

use std::{fs, process};
use std::io::{Write, BufRead};
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};
use std::collections::HashMap;
use comms::{Controllable, CmdTo, CmdFrom, Power};
use cli::CLI;
use web::Web;
use structure::Structure;
use bluefox::Bluefox;
use optoforce::Optoforce;
use stb::STB;

#[macro_use]
extern crate log;
#[macro_use]
extern crate enum_primitive;
#[macro_use]
extern crate custom_derive;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate abort_on_panic;
extern crate num;
extern crate env_logger;
extern crate hprof;
extern crate chrono;

use chrono::UTC;

/// Helper function for rxspawn! macro
fn rxspawn<T: Controllable>(reply: &Sender<CmdFrom>) -> Service {
    Service {
        name: <T as Controllable>::NAME(), // FIXME can't use macro here because of UFCS
        thread: None,
        tx: Arc::new(Mutex::new(None))
    }.start::<T>(reply)
}

/// Spawn a bunch of service threads
#[macro_export]
macro_rules! rxspawn {
    ($reply:expr; $($s:ty),*) => {
        vec![ $( rxspawn::<$s>(&$reply)),* ]
    };
}

/// Try downcasting an Any to several types, performing the same or different actions if one of the
/// types matches, otherwise do a fallback
///
/// Syntax is similar to `match`, except the types are separated by commas and the _ case comes
/// first (TODO fix this)
#[macro_export]
macro_rules! downcast {
    ($any:ident { _ => $code:expr, $($($ts:ty),+ => $codes:expr),* }) => {
        $(
            $(
                if let Some($any) = $any.downcast_ref::<$ts>() {
                    $codes
                }
            )else+
        )else*
        else { $code }
    }
}

/// Service descriptor
struct Service {
    /// short identifier
    name: &'static str,
    /// handle to running thread (actually the middle manager, see Service::start)
    thread: Option<thread::JoinHandle<()>>,
    /// synchronized Sender for commands from the master thread
    /// (should always be Some after Service::start runs)
    tx: Arc<Mutex<Option<Sender<CmdTo>>>>,
}

impl Service {
    /** Here we start two threads: the service itself and a "middle manager".
     *
     *  The middle manager's job is to watch the service and restart it if it panics.
     *  (If the service thread terminates quietly, the middle manager does the same.)
     *
     *  In case of panic, the middle manager performs three tasks:
     *
     *   - notifies the master thread using CmdFrom::Panicked
     *   - creates a new channel and replaces self.tx so the master thread doesn't notice
     *   - starts a new service thread (but does not send CmdTo::Start)
     *
     *  This modification from within the middle manager thread is the reason self.tx is such a
     *  monstrosity of containers.
     *
     *  NB: for this scheme to work, the middle manager thread must never panic!
     */
    fn start<T: Controllable>(mut self, reply: &Sender<CmdFrom>) -> Service {
        let master_tx = reply.clone();

        let name = self.name; // screw you, borrowck
        let rx_ref = self.tx.clone(); // for concurrent modification from within the middle manager thread
        self.thread = Some(thread::Builder::new()
                           .name(format!("{} middle-manager", name))
                           .spawn(move || {
                               let mut i = 0; // just for cosmetics: count service thread restarts
                               loop {
                                   i += 1;

                                   // create master => service channel
                                   //   (give receiving end to service thread, swap sending end
                                   //   into self.tx)
                                   let (thread_tx, thread_rx) = channel::<CmdTo>();
                                   // clone sending end of service => master channel
                                   let cloned_master_tx = master_tx.clone();
                                   // perform the self.tx swap
                                   *rx_ref.lock().unwrap() = Some(thread_tx);

                                   // start service thread!
                                   match thread::Builder::new()
                                       .name(format!("{} service (incarnation #{})", name, i))
                                       .spawn(move || {
                                           comms::go::<T>(thread_rx, cloned_master_tx)
                                       }).unwrap().join() {
                                           // service thread died quietly: do the same
                                           Ok(_) => return,
                                           // service thread panicked: notify master and restart
                                           Err(y) => master_tx.send(CmdFrom::Panicked {
                                               thread: name,
                                               panic_reason: downcast!(y {
                                                                            _ => format!("{:?}", y),
                                                                            String, &str => format!("{}", y)
                                                                      })
                                           }).unwrap()
                                       };
                               }
                           }).unwrap());
        self
    }
}

fn find(services: &[Service], s: String) -> Option<&Service> {
    let s = s.to_lowercase();
    services.iter().position(|x| x.name.to_lowercase() == s).map(|i| &services[i])
}

fn send_to(services: &[Service], s: String, cmd: CmdTo) -> bool {
    match find(services, s) {
        Some(srv) => {
            srv.tx.lock().unwrap().as_ref().map(|s| s.send(cmd).unwrap());
            true
        }
        None => false,
    }
}

fn start(services: &[Service], s: String) -> bool {
    send_to(services, s, CmdTo::Start)
}

fn stop(services: &[Service], s: String) -> bool {
    send_to(services, s, CmdTo::Stop)
}

fn stop_all(services: &mut [Service]) {
    for s in services {
        s.tx.lock().unwrap().as_ref().map(|s| s.send(CmdTo::Quit).unwrap());
        s.thread.take().map(|t| t.join().unwrap_or_else(|e| errorln!("Failed to join {} thread: {:?}", s.name, e)));
    }

    // TODO wait for writer thread
}

/// Main function that does everything
///
/// TODO actually use the logging infrastructure
fn main() {
    prof!("main", {

        env_logger::init().unwrap();

        info!("Hello, world!");

        let (reply_tx, reply_rx) = channel();

        let mut services = rxspawn!(reply_tx; CLI, Web, Structure, Bluefox, Optoforce, STB);
        let mut timers = HashMap::new();

        thread::sleep_ms(500); // wait for threads to start

        start(&services, "cli".to_owned());
        start(&services, "web".to_owned());

        loop {
            match reply_rx.recv() {
                Ok(cmd) => match cmd {
                    CmdFrom::Start(s, tx) => {
                        println!("STARTING {}", s);
                        tx.send(start(&services, s)).unwrap();
                    },
                    CmdFrom::Stop(s, tx)  => {
                        println!("STOPPING {}", s);
                        tx.send(stop(&services, s)).unwrap();
                    },
                    CmdFrom::Quit         => {
                        println!("STOPPING ALL");
                        stop_all(&mut services[1..]);
                        println!("EXITING");
                        break;
                    },
                    CmdFrom::Panic        => {
                        println!("KABOOM");
                        panic!("Child thread initiated panic");
                    },
                    CmdFrom::Power(power) => {
                        // step 1. delete keepalive file
                        fs::remove_file("keepalive").unwrap();

                        // step 2. stop all services
                        stop_all(&mut services[1..]);

                        // step 3. reboot system through DBUS
                        process::Command::new("dbus-send")
                            .arg("--system")
                            .arg("--print-reply")
                            .arg("--dest=org.freedesktop.login1")
                            .arg("/org/freedesktop/login1")
                            .arg(
                                match power {
                                    Power::PowerOff => "org.freedesktop.login1.Manager.PowerOff",
                                    Power::Reboot   => "org.freedesktop.login1.Manager.Reboot",
                                })
                            .arg("boolean:true")
                            .spawn().unwrap()
                            .wait().unwrap();
                    },
                    CmdFrom::Timeout { thread: who, ms: _ }  => {
                        // TODO actually time the service and do something if it times out
                        if find(&services, who.to_owned()).is_some() {
                            timers.insert(who, UTC::now());
                        } else {
                            panic!("Nonexistent service asked for timeout");
                        }
                    },
                    CmdFrom::Timein { thread: who }    => {
                        if timers.contains_key(who) {
                            println!("Service {} took {} ms", who, UTC::now() - *timers.get(who).unwrap());
                            timers.remove(who);
                        } else {
                            panic!("Timein with no matching timeout");
                        }
                    },
                    CmdFrom::Data(d)      => {
                        let mut words = d.split(' ');
                        match &*words.next().unwrap() {
                            "send" => { send_to(&services, "web".to_owned(), CmdTo::Data(d[5..].to_owned())); },
                            "kick" => { send_to(&services, words.next().unwrap().to_owned(), CmdTo::Data("kick".to_owned())); },
                            _      => { errorln!("Strange message {} received from a service", d); }
                        }
                    },
                    CmdFrom::Panicked { thread: who, panic_reason: why } => {
                        errorln!("Service {} panicked! (reason: {})", who, why);
                        send_to(&services, "web".to_owned(), CmdTo::Data(format!("panic {} {}", who, why)));
                    },
                },
                Err(_) => { stop_all(&mut services[1..]); break; }
            }
        }

        // we can't really join or kill the CLI thread, because it is waiting on stdin
        // so just exit, and it will be killed

    });

    println!("\n\n");
    hprof::profiler().print_timing();
}
