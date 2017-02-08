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
//! Note that (obviously) when the NUC is running as a hotspot, it has no internet connection. The
//! NUC's IP address is 10.42.0.1 (though I was unable to _set_ this because NetworkManager sucks,
//! so let's hope it never changes its mind).
//!
//! Set up DHCP and DNS as follows:
//!
//! 1. Install the packages: <code>sudo apt-get install isc-dhcp-server hostapd bind9</code>
//! 2. Add the following lines to <code>/etc/bind/named.conf.local</code>:
//! <pre>
//! zone "nri" {
//!     type master;
//!     file "/etc/bind/db.nri";
//! };
//! </pre>
//! 3. Create the file <code>/etc/bind/db.nri</code> with the following contents:
//! <pre>
//! $TTL	604800
//! @	IN	SOA	nri. root.nri. (
//! 			      2		; Serial
//! 			 604800		; Refresh
//! 			  86400		; Retry
//! 			2419200		; Expire
//! 			 604800 )	; Negative Cache TTL
//! ;
//! @	IN	NS	nri.
//! @	IN	A	127.0.0.1
//! @	IN	A	10.42.0.1
//! @	IN	AAAA	::1
//! </pre>
//! 4. Open <code>/etc/default/isc-dhcp-server</code> and change the line
//!    <code>INTERFACES=""</code> to <code>INTERFACES="wlan0"</code>.
//! 5. Add the following lines to <code>/etc/dhcp/dhcpd.conf</code>:
//! <pre>
//! subnet 10.42.0.0 netmask 255.255.255.0 {
//!     range 10.42.0.2 10.42.0.200;
//!     option domain-name-servers 10.42.0.1;
//!     option routers 10.42.0.1;
//! }
//! </pre>
//! 6. Restart the services bind9, isc-dhcp-server, and hostapd.
//!
//! Lastly, the server runs on port 3000. The following iptables rules will forward port 80 to port
//! 3000 so that you can simply type "http://nri" (from another computer connected to the Wi-Fi
//!      hotspot) or "http://localhost" (from the NUC itself):
//!
//! 1. <code>sudo iptables -t nat -A PREROUTING -p tcp --dport 80 -j REDIRECT --to-port 3000</code>
//! 2. <code>sudo iptables -t nat -I OUTPUT -p tcp -d 127.0.0.1 --dport 80 -j REDIRECT --to-port 3000</code>

#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
#![cfg_attr(feature = "nightly", feature(const_fn))]

#[macro_use] extern crate utils;
extern crate comms;
#[macro_use] extern crate guilt_by_association;
#[macro_use] extern crate error_chain;
extern crate scribe;
extern crate cli;
extern crate web;
extern crate teensy;
extern crate optoforce;
extern crate structure;
extern crate bluefox;
extern crate biotac;
extern crate vicon;

use std::{fs, panic, process, thread};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};
use std::collections::HashMap;
use std::time::Duration;
use comms::{Controllable, CmdTo, CmdFrom, Power};
use cli::CLI;
use web::Web;
use teensy::Teensy;
use optoforce::Optoforce;
use structure::Structure;
use bluefox::Bluefox;
use biotac::Biotac;
use vicon::Vicon;

#[macro_use] extern crate log;
extern crate env_logger;
extern crate hprof;
extern crate chrono;

use chrono::UTC;

error_chain! {
}

/// Helper function for rxspawn! macro
fn rxspawn<T: Controllable>(reply: &Sender<CmdFrom>) -> Result<Service> {
    Service::new::<T>(reply)
}

/// Spawn a bunch of service threads
#[macro_export]
macro_rules! rxspawn {
    ($reply:expr; $($s:ty),*) => {
        vec![ $( rxspawn::<$s>(&$reply)? ),* ]
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
    thread: thread::JoinHandle<()>,
    /// synchronized Sender for commands from the master thread
    /// (should always be Some after Service::start runs)
    tx: Arc<Mutex<Sender<CmdTo>>>,
}

impl Service {
    fn new<T: Controllable>(reply: &Sender<CmdFrom>) -> Result<Self> {
        let master_tx = reply.clone();
        let (tx, _) = channel();
        let tx_arc = Arc::new(Mutex::new(tx));
        let tx_ref = tx_arc.clone();
        let thread = thread::Builder::new()
            .name(format!("{} manager", guilty!(T::NAME)))
            .spawn(move || {
                loop {
                    let (thread_tx, thread_rx) = channel::<CmdTo>();
                    let cloned_master_tx = master_tx.clone();
                    *tx_ref.lock().expect("mutex poisoned") = thread_tx;

                    match panic::catch_unwind(panic::AssertUnwindSafe(|| comms::go::<T>(thread_rx, cloned_master_tx))) {
                        Ok(Ok(())) => { break }

                        Ok(Err(e)) => {
                            master_tx.send(
                                CmdFrom::Panicked {
                                    thread: guilty!(T::NAME),
                                    panic_reason: format!("{:?}", e)
                                }).expect("master is dead");
                        }

                        Err(e) => {
                            master_tx.send(
                                CmdFrom::Panicked {
                                    thread: guilty!(T::NAME),
                                    panic_reason: downcast!(e {
                                        _ => format!("{:?}", e),
                                        String, &str => e.to_string()
                                    })
                                }).expect("master is dead");
                        }
                    }
                }
            })
            .chain_err(|| "thread creation failed")?;

        Ok(Service {
            name: guilty!(T::NAME),
            thread: thread,
            tx: tx_arc
        })
    }
}

fn find(services: &[Service], s: String) -> Option<&Service> {
    let s = s.to_lowercase();
    services.iter().position(|x| x.name.to_lowercase() == s).map(|i| &services[i])
}

fn send_to(services: &[Service], s: String, cmd: CmdTo) -> Result<bool> {
    match find(services, s) {
        Some(srv) => {
            srv.tx
               .lock().expect("mutex poisoned")
               .send(cmd).expect("manager is dead");
            Ok(true)
        }
        None => Ok(false),
    }
}

fn start(services: &[Service], s: String, d: Option<String>) -> Result<bool> {
    send_to(services, s, CmdTo::Start(d))
}

fn stop(services: &[Service], s: String) -> Result<bool> {
    send_to(services, s, CmdTo::Stop)
}

fn stop_all<I: Iterator<Item=Service>>(services: I) {
    for Service { name, thread, tx } in services {
        tx.lock().expect("mutex poisoned")
          .send(CmdTo::Quit).expect("manager is dead");
        thread.join().unwrap_or_else(|e| errorln!("Failed to join {} thread: {:?}", name, e));
    }

    // TODO wait for writer thread
}

fn main() {
    if let Err(e) = try_main() {
        errorln!("ERROR: {:?}", e);

        for e in e.iter().skip(1) {
            println!("caused by: {}", e);
        }

        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }

        process::exit(1);
    }
}

/// Main function that does everything
///
/// TODO actually use the logging infrastructure
fn try_main() -> Result<()> {
    prof!("main", {

        env_logger::init().chain_err(|| "failed to set up logger")?;

        info!("Hello, world!");

        let (reply_tx, reply_rx) = channel();

        let mut services = rxspawn!(reply_tx; CLI, Web, Teensy, Optoforce, Structure, Bluefox, Optoforce, Biotac, Vicon);
        let mut timers = HashMap::new();

        thread::sleep(Duration::from_millis(500)); // wait for threads to start

        start(&services, "cli".to_owned(), None)?;
        start(&services, "web".to_owned(), None)?;

        loop {
            match reply_rx.recv() {
                Ok(cmd) => match cmd {
                    CmdFrom::Start(s, d, tx) => {
                        println!("STARTING {}", s);
                        tx.send(start(&services, s, d)?).chain_err(|| "could not send start command")?;
                    },
                    CmdFrom::Stop(s, tx)  => {
                        println!("STOPPING {}", s);
                        tx.send(stop(&services, s)?).chain_err(|| "could not send stop command")?;
                    },
                    CmdFrom::Quit         => {
                        println!("STOPPING ALL");
                        stop_all(services.drain(1..));
                        println!("EXITING");
                        break;
                    },
                    CmdFrom::Panic        => {
                        println!("KABOOM");
                        panic!("Child thread initiated panic");
                    },
                    CmdFrom::Power(power) => {
                        // step 1. delete keepalive file
                        let _ = fs::remove_file("keepalive");

                        // step 2. stop all services
                        stop_all(services.drain(1..));

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
                            .spawn().chain_err(|| "could not start dbus-send process")?
                            .wait().chain_err(|| "dbus-send process did not complete successfully")?;
                    },
                    CmdFrom::Timeout { thread: who, .. }  => {
                        // TODO actually time the service and do something if it times out
                        if find(&services, who.to_owned()).is_some() {
                            timers.insert(who, UTC::now());
                        } else {
                            bail!("Nonexistent service asked for timeout");
                        }
                    },
                    CmdFrom::Timein { thread: who }    => {
                        if let Some(then) = timers.remove(who) {
                            println!("Service {} took {} ms", who, UTC::now().signed_duration_since(then));
                        } else {
                            bail!("Timein with no matching timeout");
                        }
                    },
                    CmdFrom::Data(d)      => {
                        let mut words = d.split(' ');
                        match &*words.next().unwrap_or("") {
                            "send" => { send_to(&services, "web".to_owned(), CmdTo::Data(d[5..].to_owned()))?; },
                            "kick" => { send_to(&services, words.next().unwrap_or("").to_owned(), CmdTo::Data("kick".to_owned()))?; },
                            "to"   => { send_to(&services, words.next().unwrap_or("").to_owned(), CmdTo::Data(words.collect::<Vec<_>>().join(" ")))?; },
                            _      => { errorln!("Strange message {} received from a service", d); }
                        }
                    },
                    CmdFrom::Panicked { thread: who, panic_reason: why } => {
                        errorln!("Service {} panicked! (reason: {})", who, why);
                        send_to(&services, "web".to_owned(), CmdTo::Data(format!("panic {} {}", who, why)))?;
                    },
                },
                Err(_) => { stop_all(services.drain(1..)); break; }
            }
        }

        // we can't really join or kill the CLI thread, because it is waiting on stdin
        // so just exit, and it will be killed
        
        Ok::<(), Error>(())
    })?;

    println!("\n\n");
    hprof::profiler().print_timing();

    Ok(())
}
