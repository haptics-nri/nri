mod wrapper;

extern crate time;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::{Receiver, TryRecvError};
use super::comms::Cmd;

pub fn go(rx: Receiver<Cmd>) {
    match rx.recv() {
        Ok(cmd) => match cmd {
            Cmd::Start => {}, // let's go!
            Cmd::Stop => return, // didn't even get to start
        },
        Err(e) => return, // main thread exploded?
    }

    wrapper::initialize();
    let device = wrapper::Device::new(None).unwrap();
    let depth = wrapper::VideoStream::new(&device, wrapper::OniSensorType::Depth).unwrap();
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
    wrapper::shutdown();
    println!("{} frames grabbed in {} s ({} FPS)!", i, (end - start).num_seconds(), 1000.0*(i as f64)/((end - start).num_milliseconds() as f64));
}

