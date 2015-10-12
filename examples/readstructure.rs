#[macro_use] extern crate lazy_static;
#[macro_use] mod common;
extern crate lodepng;

use std::fmt;
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

impl common::Pixels<u16> for Row {
    fn pixel(&self, i: usize) -> u16 {
        self.pixels[i]
    }
}

fn main() {
    common::do_camera::<u16, Row>(640, 480, 1, ColorType::LCT_GREY, 16);
}

