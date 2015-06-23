//! CLI interface to view and control running services

use super::comms::{Controllable, CmdFrom};
use std::io;
use std::io::{BufRead, Write};
use std::process::Command;
use std::sync::mpsc::{channel, Sender};

/// Controllable struct for the CLI
pub struct CLI {
    tx: Sender<CmdFrom>,
}

impl Controllable for CLI {
    fn setup(tx: Sender<CmdFrom>, _: Option<String>) -> CLI {
        CLI { tx: tx }
    }

    fn step(&mut self, _: Option<String>) -> bool {
        print!("> "); io::stdout().flush();

        let stdin = io::stdin();
        let fat_line = stdin.lock().lines().next().unwrap().unwrap();
        let line = fat_line.trim();

        if line.starts_with("!") {
            Command::new("sh").args(&["-c", &line[1..]]).status();
        } else {
            let mut words = line.split(" ");
            match words.next().unwrap_or("") {
                "" => {},
                "start" => {
                    let dev = words.next().unwrap_or("");
                    if !rpc!(self.tx, CmdFrom::Start, dev.to_string()).unwrap() {
                        errorln!("Failed to start {}", dev);
                    }
                },
                "stop" => {
                    let dev = words.next().unwrap_or("");
                    if !rpc!(self.tx, CmdFrom::Stop, dev.to_string()).unwrap() {
                        errorln!("Failed to stop {}", dev);
                    }
                },
                "quit" => {
                    self.tx.send(CmdFrom::Quit);
                },
                _ => println!("Unknown command!")
            }
        }

        false
    }

    fn teardown(&mut self) {
        // this will never be called
    }
}

