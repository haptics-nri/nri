extern crate lodepng;

extern crate nri;

use std::fmt;
use std::sync::atomic::Ordering;
use lodepng::ColorType;

struct Row {
    pixels: [u16; 640]
}

impl fmt::Debug for Row {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for col in 0..640 {
            try!(write!(f, "{}, ", self.pixels[col]));
        }
        try!(write!(f, "\n"));
        Ok(())
    }
}

impl nri::Pixels<u16> for Row {
    fn pixel(&self, i: usize) -> u16 {
        self.pixels[i]
    }
}

fn main() {
    nri::VERBOSITY.store(0, Ordering::SeqCst);
    nri::do_camera::<u16, Row, _, _>("structure", |_, _, _| {}, (), 640, 480, 1, ColorType::LCT_GREY, 16);
}

