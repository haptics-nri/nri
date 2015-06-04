mod structure;
mod bluefox;
mod optoforce;

use std::io;
use std::io::{Write, BufRead};
use std::fs::File;
use std::ptr;
use std::thread;
use std::sync::mpsc;

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

enum Cmd {
    Start,
    Stop
}

fn main() {
    env_logger::init().unwrap();

    info!("Hello, world!");

    let (tx, rx) = mpsc::channel();

    let threads = vec![
        thread::spawn(move || structure(&rx))
        ];

    print!("> "); io::stdout().flush();
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(line) => match line.trim() {
                "structure" => {
                    println!("Starting thread");
                    tx.send(Cmd::Start);
                },
                "quit" => {
                    println!("Stopping threads");
                    tx.send(Cmd::Stop);
                    break;
                },
                _ => println!("Unknown command!")
            },
            Err(e) => panic!("Main thread IO error: {}", e)
        }
        print!("> "); io::stdout().flush();
    }

    for t in threads {
        t.join().unwrap();
    }
}

fn structure(rx: &mpsc::Receiver<Cmd>) {
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
                mpsc::TryRecvError::Empty => {}, // continue
                mpsc::TryRecvError::Disconnected => break, // main thread exploded?
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
