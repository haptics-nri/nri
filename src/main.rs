mod web;
mod structure;
mod bluefox;
mod optoforce;
mod mpmc;
mod comms;

use std::io;
use std::io::{Write, BufRead};
use std::ptr;
use std::thread;
use std::sync::mpsc::{Receiver, TryRecvError};
use mpmc::MultiSender;
use comms::Cmd;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate libc;

macro_rules! errorln(
    ($($arg:tt)*) => (
        match writeln!(&mut ::std::io::stderr(), $($arg)* ) {
            Ok(_) => {},
            Err(x) => panic!("Unable to write to stderr: {}", x),
        }
    )
);

fn main() {
    env_logger::init().unwrap();

    info!("Hello, world!");

    let mut ansible = MultiSender::new();

    let names = vec!["web", "structure"];
    let threads = vec![
        { let rx = ansible.receiver(); thread::spawn(move || web::go(rx)) }      ,
        { let rx = ansible.receiver(); thread::spawn(move || structure::go(rx)) },
        ];

    print!("> "); io::stdout().flush();
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(line) => {
                let mut words = line.trim().split(" ");
                match words.next().unwrap_or("") {
                    "" => {},
                    "start" => {
                        let dev = words.next().unwrap_or("");
                        match names.iter().position(|x| *x == dev) {
                            Some(i) => {
                                println!("Starting thread for {} ({})", i, dev);
                                ansible.send_one(i, Cmd::Start);
                            },
                            None => println!("Start what now?"),
                        }
                    },
                    "stop" => {
                        let dev = words.next().unwrap_or("");
                        match names.iter().position(|x| *x == dev) {
                            Some(i) => {
                                println!("Stopping thread for {} ({})", i, dev);
                                ansible.send_one(i, Cmd::Stop);
                            },
                            None => println!("Stop what now?"),
                        }
                    },
                    "quit" => {
                        println!("Stopping threads");
                        ansible.send(Cmd::Stop);
                        break;
                    },
                    _ => println!("Unknown command!")
                }
            },
            Err(e) => panic!("Main thread IO error: {}", e)
        }
        print!("> "); io::stdout().flush();
    }

    for t in threads {
        t.join().unwrap();
    }
}

