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
//! ```text
//! nri$ cargo doc
//! ```
//! - Now we need the rustdoc command for this crate.
//!
//! ```text
//! nri$ touch src/main.rs
//! nri$ cargo doc -v | grep rustdoc | awk "-FRunning " '{print substr($NF, 2, length($NF)-2)}' > cargo-doc-command
//! ```
//! - Edit <tt>cargo-doc-command</tt> and add <tt>--no-defaults --passes "collapse-docs" --passes "unindent-comments"</tt> after <tt>src/main.rs</tt>. Then run it.
//!
//! ```text
//! nri$ source cargo-doc-command
//! ```
//! - Copy the docs to the Github Pages repo, commit, and push.
//!
//! ```text
//! nri$ rsync -a target/doc ../haptics-nri.github.io
//! nri$ cd ../haptics-nri.github.io
//! haptics-nri.github.io$ git add doc
//! haptics-nri.github.io$ git commit -m "cargo doc"
//! haptics-nri.github.io$ git push
//! ```
//! - The docs are now live (after 30s or so) at http://haptics-nri.github.io/doc/nri.
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
//! 8. Edit the file /etc/NetworkManager/system-connections/<SSID> and change
//!    <tt>mode=infrastructure</tt> to <tt>mode=ap</tt>.
//! 9. Deactivate and reactivate Wi-Fi. Then you should be able to select the new SSID to
//!    "connect". And then you should be able to connect from other devices!
//!
//! Note that (obviously) when the NUC is running as a hotspot, it has no internet connection. I
//! tried to get DNS running, but I failed, so for now you have to use an IP address to access the
//! NUC. You can see what it chose by running <tt>ip a</tt> -- it seems to like 10.42.0.1.

mod web;
mod structure;
mod bluefox;
mod optoforce;
mod comms;
mod mpmc;

use std::io;
use std::io::{Write, BufRead};
use std::ascii::AsciiExt;
use std::ptr;
use std::thread;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::process::Command;
use mpmc::MultiSender;
use comms::Cmd;
use web::Web;
use structure::Structure;
use bluefox::Bluefox;
use optoforce::Optoforce;

#[macro_use]
extern crate log;
extern crate env_logger;

macro_rules! errorln(
    ($($arg:tt)*) => (
        match writeln!(&mut ::std::io::stderr(), $($arg)* ) {
            Ok(_) => {},
            Err(x) => panic!("Unable to write to stderr: {}", x),
        }
    )
);

macro_rules! rxspawn {
    ($ansible:expr; $($s:ty),*) => {
        vec![
            $(
                {
                    let rx = $ansible.receiver();
                    Service {
                        name: stringify!($s).to_ascii_lowercase(),
                        thread: thread::spawn(move || comms::go::<$s>(rx))
                    }
                }
            ),*
        ]
    };
}

/// Service descriptor
struct Service {
    name: String,
    thread: thread::JoinHandle<()>,
}

/// Main function that does everything
///
/// TODO split out CLI interface into a mod
/// TODO actually use the logging infrastructure
fn main() {
    env_logger::init().unwrap();

    info!("Hello, world!");

    let mut ansible = MultiSender::new();

    let services = rxspawn!(ansible; Web, Structure, Bluefox, Optoforce);

    print!("> "); io::stdout().flush();
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(line) => {
                let line = line.trim();
                if line.starts_with("!") {
                    Command::new("sh").args(&["-c", &line[1..]]).status();
                } else {
                    let mut words = line.split(" ");
                    match words.next().unwrap_or("") {
                        "" => {},
                        "start" => {
                            let dev = words.next().unwrap_or("");
                            match services.iter().position(|x| x.name == dev) {
                                Some(i) => {
                                    println!("Starting thread for {} ({})", i, dev);
                                    ansible.send_one(i, Cmd::Start);
                                },
                                None => println!("Start what now?"),
                            }
                        },
                        "stop" => {
                            let dev = words.next().unwrap_or("");
                            match services.iter().position(|x| x.name == dev) {
                                Some(i) => {
                                    println!("Stopping thread for {} ({})", i, dev);
                                    ansible.send_one(i, Cmd::Stop);
                                },
                                None => println!("Stop what now?"),
                            }
                        },
                        "quit" => {
                            println!("Stopping threads");
                            ansible.send(Cmd::Quit);
                            break;
                        },
                        _ => println!("Unknown command!")
                    }
                }
            },
            Err(e) => panic!("Main thread IO error: {}", e)
        }
        print!("> "); io::stdout().flush();
    }

    for s in services {
        s.thread.join().unwrap();
    }
}

