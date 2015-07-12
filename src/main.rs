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
macro_rules! errorln {
    ($($arg:tt)*) => {{
        use std::io::Write;
        match writeln!(&mut ::std::io::stderr(), $($arg)* ) {
            Ok(_) => {},
            Err(x) => panic!("Unable to write to stderr: {}", x),
        }
    }}
}

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

#[macro_use] extern crate guilt_by_association;

// TODO move this profiling stuff to a mod
use std::cell::RefCell;

thread_local!(static PROF: RefCell<Option<hprof::Profiler>> = RefCell::new(None));

macro_rules! prof {
    ($b:expr) => { prof!(stringify!($b), $b) };
    //($n:expr, $b:expr) => ($b)
    ($n:expr, $b:expr) => {
        $crate::PROF.with(|wrapped_prof| {
            let appease_borrowck = wrapped_prof.borrow();
            let g = match *appease_borrowck {
                Some(ref prof) => prof.enter($n),
                None => $crate::hprof::enter($n)
            };
            let ret = { $b };
            drop(g);
            ret
        })
    }
}

#[macro_use] mod comms;
mod cli;
mod web;
mod structure;
mod bluefox;
mod optoforce;
mod stb;

use std::io::{Write, BufRead};
use std::ascii::AsciiExt;
use std::thread;
use std::sync::mpsc::{channel, Sender};
use std::collections::HashMap;
use comms::{CmdTo, CmdFrom};
use cli::CLI;
use web::Web;
use structure::Structure;
use bluefox::Bluefox;
use optoforce::Optoforce;
use stb::STB;

#[macro_use] extern crate log;
#[macro_use] extern crate enum_primitive;
extern crate num;
extern crate env_logger;
extern crate hprof;
extern crate chrono;

use chrono::UTC;

macro_rules! rxspawn {
    ($reply:expr; $($s:ty),*) => {
        vec![
            $(
                {
                    let (thread_tx, thread_rx) = channel();
                    let master_tx = $reply.clone();
                    Service {
                        name: stringify!($s).to_ascii_lowercase(),
                        thread: Some(thread::spawn(move || comms::go::<$s>(thread_rx, master_tx))),
                        tx: thread_tx
                    }
                }
            ),*
        ]
    };
}

/// Service descriptor
struct Service {
    name: String,
    thread: Option<thread::JoinHandle<()>>,
    tx: Sender<CmdTo>,
}

fn find(services: &[Service], s: String) -> Option<&Service> {
    services.iter().position(|x| x.name == s).map(|i| &services[i])
}

fn send_to(services: &[Service], s: String, cmd: CmdTo) -> bool {
    match find(services, s) {
        Some(srv) => {
            srv.tx.send(cmd).unwrap();
            true
        },
        None => false 
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
        s.tx.send(CmdTo::Quit).unwrap();
        s.thread.take().map(|t| t.join().unwrap_or_else(|e| errorln!("Failed to join {} thread: {:?}", s.name, e)));
    }
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

        start(&services, "cli".to_string());
        start(&services, "web".to_string());

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
                    CmdFrom::Timeout(n, ms)  => {
                        // TODO actually time the service and do something if it times out
                        if find(&services, n.to_string()).is_some() {
                            timers.insert(n, UTC::now());
                        } else {
                            panic!("Nonexistent service asked for timeout");
                        }
                    },
                    CmdFrom::Timein(n)    => {
                        if timers.contains_key(n) {
                            println!("Service {} took {} ms", n, UTC::now() - *timers.get(n).unwrap());
                            timers.remove(n);
                        } else {
                            panic!("Timein with no matching timeout");
                        }
                    },
                    CmdFrom::Data(d)      => {
                        let mut words = d.split(' ');
                        match &*words.next().unwrap() {
                            "structure" | "bluefox" => { send_to(&services, "web".to_string(), CmdTo::Data(d.to_owned())); },
                            "kick"                  => { send_to(&services, words.next().unwrap().to_string(), CmdTo::Data("kick".to_string())); },
                            _                       => { errorln!("Strange message {} received from a service", d); }
                        }
                    }
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

