extern crate time;

#[macro_use] mod common;

use std::fmt;

struct Data {
    stamp: time::Timespec,
    xyz: [f64; 3]
}

impl fmt::Debug for Data {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        try!(write!(f, "{:.9}, {:.9}, {:.9}, {:.9}",
                    self.stamp.sec as f64 + self.stamp.nsec as f64 / 1_000_000_000f64,
                    self.xyz[0], self.xyz[1], self.xyz[2]));
        Ok(())
    }
}

fn main() {
    common::read_binary::<Data>("Timestamp, X, Y, Z");
}

