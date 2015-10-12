#[macro_use] extern crate lazy_static;
#[macro_use] mod common;

use std::fmt;

struct Row {
    pixels: [[u8; 3]; 1600]
}

impl fmt::Debug for Row {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        Err(fmt::Error)
    }
}

impl common::Pixels<[u8; 3]> for Row {
    fn pixel(&self, i: usize) -> [u8; 3] {
        self.pixels[i]
    }
}

fn main() {
    common::do_camera::<[u8; 3], Row>(1600, 1200, 3);
}

