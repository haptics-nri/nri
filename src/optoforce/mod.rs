//! Service to read data from the OptoForce sensor

use super::comms::Controllable;

pub struct Optoforce;

impl Controllable<Optoforce> for Optoforce {
    fn setup() -> Optoforce {
        Optoforce
    }

    fn step(&mut self) -> bool {
        true
    }

    fn teardown(&mut self) {
    }
}


