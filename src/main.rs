mod web;
mod structure;
mod bluefox;
mod optoforce;
mod mpmc;
mod comms;

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

struct Service {
    name: String,
    thread: thread::JoinHandle<()>,
}

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

