//! Service to read data from the OptoForce sensor

use super::comms::{Controllable, CmdFrom};
use std::sync::mpsc::{channel, Sender};

stub!(Optoforce);


