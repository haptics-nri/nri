mod structure;
mod bluefox;
mod optoforce;
mod mpmc;

use std::io;
use std::io::{Write, BufRead};
use std::fs::File;
use std::ptr;
use std::thread;
use std::sync::mpsc::{Receiver, TryRecvError};
use mpmc::MultiSender;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate libc;
extern crate time;

macro_rules! errorln(
    ($($arg:tt)*) => (
        match writeln!(&mut ::std::io::stderr(), $($arg)* ) {
            Ok(_) => {},
            Err(x) => panic!("Unable to write to stderr: {}", x),
        }
    )
);

#[derive(Clone)]
enum Cmd {
    Start,
    Stop
}

fn main() {
    env_logger::init().unwrap();

    info!("Hello, world!");

    let mut ansible = MultiSender::new();

    let names = vec!["structure"];
    let threads = vec![
        { let rx = ansible.receiver(); thread::spawn(move || structure(rx)) }
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
                                println!("Starting thread for device {} ({})", i, dev);
                                ansible.send_one(i, Cmd::Start);
                            },
                            None => println!("No such device!"),
                        }
                    },
                    "stop" => {
                        let dev = words.next().unwrap_or("");
                        match names.iter().position(|x| *x == dev) {
                            Some(i) => {
                                println!("Stopping thread for device {} ({})", i, dev);
                                ansible.send_one(i, Cmd::Stop);
                            },
                            None => println!("No such device!"),
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

fn structure(rx: Receiver<Cmd>) {
    match rx.recv() {
        Ok(cmd) => match cmd {
            Cmd::Start => {}, // let's go!
            Cmd::Stop => return, // didn't even get to start
        },
        Err(e) => return, // main thread exploded?
    }

    structure::wrapper::initialize();
    let device = structure::wrapper::Device::new(None).unwrap();
    let depth = structure::wrapper::VideoStream::new(&device, structure::wrapper::OniSensorType::Depth).unwrap();
    println!("device = {:?}", device);
    println!("depth = {:?}", depth);
    depth.start();
    let start = time::now();
    let mut i = 0;
    loop {
        match rx.try_recv() {
            Ok(cmd) => match cmd {
                Cmd::Start => {}, // already started
                Cmd::Stop => break, // shutdown command
            },
            Err(e) => match e {
                TryRecvError::Empty => {}, // continue
                TryRecvError::Disconnected => break, // main thread exploded?
            },
        }

        i += 1;

        let frame = depth.readFrame().unwrap();
        let data: &[u8] = frame.data();

        let mut f = File::create(format!("frame{}.dat", i)).unwrap();
        f.write_all(data);
        /*for y in 0..frame.height {
            for x in 0..frame.width {
                f.write(format!("{}", data[(y*frame.width + x) as usize]).as_bytes());
                if x == frame.width-1 {
                    f.write(b"\n");
                } else {
                    f.write(b",");
                }
            }
        }*/
    }
    let end = time::now();
    depth.stop();
    depth.destroy();
    structure::wrapper::shutdown();
    println!("{} frames grabbed in {} s ({} FPS)!", i, (end - start).num_seconds(), 1000.0*(i as f64)/((end - start).num_milliseconds() as f64));
}
