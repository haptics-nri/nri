//! Service to read data from the OptoForce sensor

use super::comms::{Controllable, CmdFrom};
use std::sync::mpsc::{channel, Sender};

/// Controllable struct for the sensor
pub struct Optoforce;

impl Controllable<Optoforce> for Optoforce {
    fn setup() -> Optoforce {
        Optoforce
    }

    fn step(&mut self, tx: Sender<CmdFrom>) -> bool {
        true
    }

    fn teardown(&mut self) {
    }
}


