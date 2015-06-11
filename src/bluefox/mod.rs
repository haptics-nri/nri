//! Service to capture frames from the mvBlueFOX3 camera

mod wrapper;

extern crate time;
use std::fs::File;
use std::io::Write;
use super::comms::Controllable;

/// Controllable struct for the camera
pub struct Bluefox {
    /// Private device handle
    device: wrapper::Device,

    /// Time that setup() was last called (used for calculating frame rates)
    start: time::Tm,

    /// Number of frames captured since setup() was last called (used for calculating frame rates)
    i: usize,
}

impl Controllable<Bluefox> for Bluefox {
    fn setup() -> Bluefox {
        let device = wrapper::Device::new().unwrap();
        //device.request_reset();
        
        Bluefox { device: device, i: 0, start: time::now() }
    }

    fn step(&mut self) -> bool {
        self.i += 1;

        let image = self.device.request().unwrap();
        println!("got frame #{}, a {:?} image in {:?} format", self.i, image.size(), image.format());

        let mut f = File::create(format!("bluefox{}.dat", self.i)).unwrap();
        f.write_all(image.data());

        false
    }

    fn teardown(&mut self) {
        let end = time::now();
        //device.request_reset();
        self.device.close();
        println!("{} frames grabbed in {} s ({} FPS)!", self.i, (end - self.start).num_seconds(), 1000.0*(self.i as f64)/((end - self.start).num_milliseconds() as f64));
    }
}

