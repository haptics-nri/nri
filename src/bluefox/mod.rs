mod wrapper;

extern crate time;
use super::comms::Controllable;

pub struct Bluefox {
    device: wrapper::Device,
    start: time::Tm,
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
        println!("got frame #{}, a {}x{} {}-ch bluefox image", self.i, image.height, image.width, image.channel_count);

        false
    }

    fn teardown(&mut self) {
        let end = time::now();
        //device.request_reset();
        self.device.close();
        println!("{} frames grabbed in {} s ({} FPS)!", self.i, (end - self.start).num_seconds(), 1000.0*(self.i as f64)/((end - self.start).num_milliseconds() as f64));
    }
}

