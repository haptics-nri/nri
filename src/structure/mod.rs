mod wrapper;

extern crate time;
use std::fs::File;
use std::io::Write;
use super::comms::Controllable;
use std::ptr;

pub struct Structure {
    device: wrapper::Device,
    depth: wrapper::VideoStream,
    start: time::Tm,
    i: usize,
}

impl Controllable<Structure> for Structure {
    fn setup() -> Structure {
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

    fn step(&mut self) {
        self.i += 1;

        let frame = self.depth.readFrame().unwrap();
        let data: &[u8] = frame.data();

        let mut f = File::create(format!("frame{}.dat", self.i)).unwrap();
        f.write_all(data);
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

