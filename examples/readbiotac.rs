#[macro_use] extern crate lazy_static;
extern crate time;

#[macro_use] mod common;

use std::fmt;
use std::iter::once;

#[repr(packed)]
struct Packet {
    stamp: time::Timespec,
    pdc: u32,
    pac: [u32; 22],
    tdc: u32,
    tac: u32,
    electrode: [u32; 19],
}

impl fmt::Debug for Packet {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        try!(write!(f, "{:.9}, ",
                    self.stamp.sec as f64 + self.stamp.nsec as f64 / 1_000_000_000f64));
        try!(write!(f, "{}, ",
                    self.pdc));
        for i in 0..22 {
            try!(write!(f, "{}, ",
                        self.pac[i]));
        }
        try!(write!(f, "{}, ",
                    self.tdc));
        try!(write!(f, "{}, ",
                    self.tac));
        for i in 0..18 {
            try!(write!(f, "{}, ",
                        self.electrode[i]));
        }
        try!(write!(f, "{}",
                    self.electrode[18]));
        Ok(())
    }
}

fn main() {
    let s: String = 
        once(String::from("Timestamp"))
        .chain(once(String::from("PDC")))
        .chain((0..22).map(|i| format!("PAC #{}", i)))
        .chain(once(String::from("TDC")))
        .chain(once(String::from("TAC")))
        .chain((0..19).map(|i| format!("Electrode #{}", i)))
        .collect::<Vec<String>>()
        .join(", ");
    common::read_binary::<Packet>(&s);
}

