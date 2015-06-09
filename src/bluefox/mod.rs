mod wrapper;

use super::comms::Controllable;

pub struct Bluefox {
    device: wrapper::Device,
}

impl Controllable<Bluefox> for Bluefox {
    fn setup() -> Bluefox {
        let device = wrapper::Device::new().unwrap();

        Bluefox { device: device }
    }

    fn step(&mut self) -> bool {
        true
    }

    fn teardown(&mut self) {
    }
}

