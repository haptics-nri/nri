mod structure;
mod bluefox;
mod optoforce;

use std::io::Write;
use std::ptr;

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

    structure::wrapper::initialize();
    let device = structure::wrapper::Device::new(structure::wrapper::ANY_DEVICE).unwrap();
    let depth = structure::wrapper::VideoStream::new(device, structure::wrapper::OniSensorType::Depth).unwrap();
    depth.start();
    loop {
        let frame = depth.readFrame();
    }
    depth.stop();
    depth.destroy();
    structure::wrapper::shutdown();
}
