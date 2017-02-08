//! CLI interface to view and control running services

#[macro_use] extern crate utils;
#[macro_use] extern crate comms;
extern crate flow;
extern crate scribe;
extern crate teensy;

#[macro_use] extern crate guilt_by_association;

extern crate chrono;

use comms::{Controllable, CmdFrom, Power, Block};
use flow::{FLOWS, Comms};
use teensy::ParkState;
use std::{env, fs, thread};
use std::io::{self, BufRead, Write};
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::time::Duration;

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
                    let mut words = command.trim().split_whitespace(); // TODO use shell quoting
                    match words.next().unwrap_or("") {
                        "" => {},
                        "episode" =>
                            if let Some(surface) = words.next() {
                                if let Some(sec_str) = words.next() {
                                    if let Ok(sec) = sec_str.parse::<u64>() {
                                        self.episode(surface, sec);
                                    } else {
                                        errorln!("Invalid duration value (episode <surface> <sec>)");
                                    }
                                } else {
                                    errorln!("No duration (episode <surface> <sec>)");
                                }
                            } else {
                                errorln!("No surface (episode <surface> <sec>)");
                            },
                        "flow" =>
                            if let Some(flowname) = words.next() {
                                self.flow(Some(flowname));
                            } else {
                                self.flow(None);
                            },
                        "cd" => { env::set_current_dir(words.next().unwrap()).unwrap(); },
                        "sleep" => {
                            if let Some(ms_str) = words.next() {
                                if let Ok(ms) = ms_str.parse::<u64>() {
                                    self.sleep(ms);
                                } else {
                                    errorln!("Invalid millisecond value (sleep <ms>)");
                                }
                            } else {
                                errorln!("No millisecond value (sleep <ms>)");
                            }
                        },
                        "start" => {
                            while let Some(dev) = words.next() {
                                let mut split = dev.splitn(2, '/');
                                self.start(split.next().unwrap(), split.next());
                            }
                        },
                        "stop" => {
                            while let Some(dev) = words.next() {
                                self.stop(dev);
                            }
                        },
                        "status" => {
                            println!("parked: {:?}", teensy::ParkState::metermaid());
                            println!("scribe: {:?}", scribe::COUNT.load(Ordering::SeqCst));
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
                        "data" => {
                            self.tx.send(CmdFrom::Data(words.collect::<Vec<_>>().join(" "))).unwrap();
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

impl CLI {
    fn start(&self, dev: &str, data: Option<&str>) {
        if !rpc!(self.tx, CmdFrom::Start, dev.to_owned(), data.map(|s| s.to_owned())).unwrap() {
            errorln!("Failed to start {}", dev);
        }
    }

    fn stop(&self, dev: &str) {
        if !rpc!(self.tx, CmdFrom::Stop, dev.to_owned()).unwrap() {
            errorln!("Failed to stop {}", dev);
        }
    }

    fn sleep(&self, ms: u64) {
        thread::sleep(Duration::from_millis(ms));
    }

    fn episode(&self, surface: &str, sec: u64) {
        match teensy::ParkState::metermaid() {
            None => errorln!("Failed to read end-effector state"),
            Some(teensy::ParkState::None) => errorln!("No end-effector"),
            Some(teensy::ParkState::Multiple) => errorln!("Multiple end-effectors"),
            Some(endeff) =>
                if let Ok(_) = env::set_current_dir("data") {
                    let datedir = chrono::Local::today().format("%Y%m%d").to_string();
                    loop {
                        if let Ok(_) = env::set_current_dir(&datedir) {
                            let mut epnum = 1;
                            for entry in fs::read_dir(".").expect("list episode dir") {
                                if let Ok(entry) = entry {
                                    if let Ok(typ) = entry.file_type() {
                                        if typ.is_dir() {
                                            if let Ok(name) = entry.file_name().into_string() {
                                                if name.starts_with(surface) && name.ends_with(endeff.short()) {
                                                    let a = surface.len();
                                                    let b = name.len() - endeff.short().len();
                                                    let num = name[a..b].parse::<u64>().unwrap() + 1;
                                                    if num > epnum {
                                                        epnum = num;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            let epdir = format!("{}{}{}", surface, epnum, endeff.short());
                            if let Ok(_) = fs::create_dir(&epdir) {
                                if let Ok(_) = env::set_current_dir(&epdir) {
                                    match endeff {
                                        teensy::ParkState::OptoForce => self.start("optoforce", None),
                                        teensy::ParkState::BioTac    => self.start("biotac", None),
                                        teensy::ParkState::Stick     => {},
                                        _ => unreachable!() // checked above
                                    }
                                    self.start("teensy", None);
                                    self.sleep(sec * 1000);
                                    self.stop("teensy");
                                    match endeff { // TODO RAII
                                        teensy::ParkState::OptoForce => self.stop("optoforce"),
                                        teensy::ParkState::BioTac    => self.stop("biotac"),
                                        teensy::ParkState::Stick     => {},
                                        _ => unreachable!() // checked above
                                    }

                                    env::set_current_dir("..").expect("leave episode dir");
                                    println!("Success!");
                                } else {
                                    errorln!("Failed to enter episode directory");
                                }
                            } else {
                                errorln!("Failed to create episode directory");
                            }

                            env::set_current_dir("../..").expect("leave episode dir");
                            break;
                        } else if let Ok(_) = fs::create_dir(&datedir) {
                            continue;
                        } else {
                            errorln!("Failed to create/enter date directory");
                            break;
                        }
                    }
                } else {
                    errorln!("No data directory");
                }
        }
    }

    fn flow(&self, name: Option<&str>) {
        #[derive(Clone)] struct CLIComms;
        impl Comms for CLIComms {
            fn print(&self, _: String) {
                /* quiet */
            }

            fn send(&self, msg: String) {
                print!("\t{} ", &msg[4..]);
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut String::new()).unwrap();
            }

            fn rpc<T, F: Fn(String) -> Result<T, String>>(&self, prompt: String, validator: F) -> T {
                loop {
                    print!("\t{}: ", &prompt[7..]);
                    io::stdout().flush().unwrap();
                    let mut input = String::new();
                    io::stdin().read_line(&mut input).unwrap();
                    match validator(input.clone()) {
                        Ok(ret) => return ret,
                        Err(msg) => println!("\t{}", &msg[7..])
                    }
                }
            }
        }

        if let Some(name) = name {
            let mut locked_flows = FLOWS.write().unwrap();
            if let Some(found) = locked_flows.get_mut(&*name) {
                loop {
                    match found.run(ParkState::None, &self.tx, CLIComms) {
                        Ok(flow::EventContour::Finishing) => break,
                        Ok(_) => continue,
                        Err(e) => { println!("Error while running flow: {:?}", e); break; }
                    }
                }
            } else {
                println!("\tERROR: flow \"{}\" not found!", name);
            }
        } else {
            let locked_flows = FLOWS.read().unwrap();
            for (_, flow) in locked_flows.iter() {
                println!("\t{} => {}", flow.shortname, flow.name);
            }
        }
    }
}

