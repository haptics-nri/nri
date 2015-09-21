//! CLI interface to view and control running services

use super::comms::{Controllable, CmdFrom, Power, Block};
use std::thread;
use std::io::{self, BufRead, Write};
use std::process::Command;
use std::sync::mpsc::Sender;

/// Controllable struct for the CLI
pub struct CLI {
    tx: Sender<CmdFrom>,
}

guilty!{
    impl Controllable for CLI {
        const NAME: &'static str = "cli",
        const BLOCK: Block = Block::Immediate,

        fn setup(tx: Sender<CmdFrom>, _: Option<String>) -> CLI {
            CLI { tx: tx }
        }

        fn step(&mut self, _: Option<String>) {
            print!("> ");
            io::stdout().flush().unwrap();

            let stdin = io::stdin();
            let fat_line = stdin.lock().lines().next().unwrap().unwrap();
            let line = fat_line.trim();

            if line.starts_with("!") {
                Command::new("sh").args(&["-c", &line[1..]]).status().unwrap();
            } else {
                for command in line.split(";") {
                    let mut words = command.trim().split(" ");
                    match words.next().unwrap_or("") {
                        "" => {},
                        "sleep" => {
                            if let Some(ms_str) = words.next() {
                                if let Ok(ms) = ms_str.parse::<u32>() {
                                    thread::sleep_ms(ms);
                                } else {
                                    errorln!("Invalid millisecond value");
                                }
                            } else {
                                errorln!("No millisecond value");
                            }
                        },
                        "start" => {
                            while let Some(dev) = words.next() {
                                if !rpc!(self.tx, CmdFrom::Start, dev.to_owned()).unwrap() {
                                    errorln!("Failed to start {}", dev);
                                }
                            }
                        },
                        "stop" => {
                            while let Some(dev) = words.next() {
                                if !rpc!(self.tx, CmdFrom::Stop, dev.to_owned()).unwrap() {
                                    errorln!("Failed to stop {}", dev);
                                }
                            }
                        },
                        "status" => {
                            println!("{:?}", super::teensy::ParkState::metermaid());
                        },
                        "quit" => {
                            self.tx.send(CmdFrom::Quit).unwrap();
                        },
                        "panic" => {
                            self.tx.send(CmdFrom::Panic).unwrap();
                        },
                        "reboot" => {
                            self.tx.send(CmdFrom::Power(Power::Reboot)).unwrap();
                        }
                        "poweroff" => {
                            self.tx.send(CmdFrom::Power(Power::PowerOff)).unwrap();
                        }
                        "websend" => {
                            self.tx.send(CmdFrom::Data("send test".to_owned())).unwrap();
                        },
                        _ => println!("Unknown command!")
                    }
                }
            }
        }

        fn teardown(&mut self) {
            // this will never be called
        }
    }
}
