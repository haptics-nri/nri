//! Service to capture frames from the Structure Sensor

#[cfg(target_os = "linux")]
mod wrapper;

extern crate time;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::{channel, Sender};
use super::comms::{Controllable, CmdFrom};

#[cfg(target_os = "linux")]
/// Controllable struct for the camera
pub struct Structure {
    /// Private handle to the device
    device: wrapper::Device,

    /// Private handle to the data stream
    depth: wrapper::VideoStream,

    /// Time that setup() was last called (used for calculating frame rates)
    start: time::Tm,

    /// Number of frames captured since setup() was last called (used for calculating frame rates)
    i: usize,
}

#[cfg(target_os = "linux")]
impl Controllable for Structure {
    fn setup(tx: Sender<CmdFrom>) -> Structure {
        wrapper::initialize();
        let device = wrapper::Device::new(None).unwrap();
        let depth = wrapper::VideoStream::new(&device, wrapper::OniSensorType::Depth).unwrap();
        println!("device = {:?}", device);
        println!("depth = {:?}", depth);
        depth.start();
        let start = time::now();
        let i = 0;
        Structure { device: device, depth: depth, start: start, i: i}
    }

    fn step(&mut self) -> bool {
        self.i += 1;

        let frame = self.depth.readFrame().unwrap();
        let data: &[u8] = frame.data();

        let mut f = File::create(format!("structure{}.dat", self.i)).unwrap();
        f.write_all(data);

        false
    }

    fn teardown(&mut self) {
        let end = time::now();
        self.depth.stop();
        self.depth.destroy();
        self.device.close();
        wrapper::shutdown();
        println!("{} frames grabbed in {} s ({} FPS)!", self.i, (end - self.start).num_seconds(), 1000.0*(self.i as f64)/((end - self.start).num_milliseconds() as f64));
    }
}

#[cfg(not(target_os = "linux"))]
stub!(Structure);

