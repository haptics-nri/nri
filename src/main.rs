mod structure;
mod bluefox;
mod optoforce;

use std::io::Write;
use std::fs::File;
use std::ptr;

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

fn main() {
    env_logger::init().unwrap();

    info!("Hello, world!");

    structure::wrapper::initialize();
    let device = structure::wrapper::Device::new(None).unwrap();
    let depth = structure::wrapper::VideoStream::new(&device, structure::wrapper::OniSensorType::Depth).unwrap();
    println!("device = {:?}", device);
    println!("depth = {:?}", depth);
    depth.start();
    let N = 10;
    let start = time::now();
    for i in 0..N {
        println!("waiting for frame {}...", i);
        let frame = depth.readFrame().unwrap();
        println!("got frame");
        let data: &[u16] = frame.data();
        println!("extracted data");
        println!("frame = {:?}", *frame);

        let mut f = File::create(format!("frame{}.csv", i)).unwrap();
        for y in 0..frame.height {
            for x in 0..frame.width {
                f.write(format!("{}", data[(y*frame.width + x) as usize]).as_bytes());
                if x == frame.width-1 {
                    f.write(b"\n");
                } else {
                    f.write(b",");
                }
            }
        }
    }
    let end = time::now();
    depth.stop();
    depth.destroy();
    structure::wrapper::shutdown();
    println!("{} frames grabbed in {} s ({} FPS)!", N, (end - start).num_seconds(), 1000.0*(N as f64)/((end - start).num_milliseconds() as f64));
}
