#[macro_use] extern crate lazy_static;
extern crate time;

extern crate nri;

use std::sync::RwLock;
use std::{fmt, env};
use std::path::Path;

#[derive(Copy, Clone)]
enum OutputMode {
    FT,
    ACC,
    GYRO,
    MAG,
}
use OutputMode::*;

lazy_static! {
    static ref OUTPUT_MODE: RwLock<OutputMode> = RwLock::new(FT);
}

#[repr(packed)]
struct XYZ<T> {
    x: T,
    y: T,
    z: T
}
#[repr(packed)]
struct Packet {
    stamp:  time::Timespec,
    dt:     u32,
    ft:     [u8; 30],
    count:  u8,
    n_acc:  u8,
    n_gyro: u8,
    imu:    [XYZ<i16>; 63]
}

impl fmt::Debug for Packet {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let a = self.n_acc as usize;
        let g = self.n_gyro as usize;

        match *OUTPUT_MODE.read().unwrap() {
            FT => {
                try!(write!(f, "{:.9}, {}, {}",
                            self.stamp.sec as f64 + self.stamp.nsec as f64 / 1_000_000_000f64,
                            self.dt,
                            self.count));
                for i in 0..30 {
                    try!(write!(f, ", {}", self.ft[i]));
                }
            },
            ACC => {
                for i in 0..a {
                    try!(write!(f, "{:.9}, {}, {}, {}, {}",
                                self.stamp.sec as f64 + self.stamp.nsec as f64 / 1_000_000_000f64,
                                i,
                                self.imu[i].x,
                                self.imu[i].y,
                                self.imu[i].z));
                    if i != a-1 {
                        try!(write!(f, "\n"));
                    }
                }
            },
            GYRO => {
                for i in 0..g {
                    try!(write!(f, "{:.9}, {}, {}, {}, {}",
                                self.stamp.sec as f64 + self.stamp.nsec as f64 / 1_000_000_000f64,
                                i,
                                self.imu[i + a].x,
                                self.imu[i + a].y,
                                self.imu[i + a].z));
                    if i != g-1 {
                        try!(write!(f, "\n"));
                    }
                }
            },
            MAG => {
                if a + g > 0 {
                    try!(write!(f, "{:.9}, {}, {}, {}",
                                self.stamp.sec as f64 + self.stamp.nsec as f64 / 1_000_000_000f64,
                                i16::from_be(self.imu[a + g].x),
                                i16::from_be(self.imu[a + g].y),
                                i16::from_be(self.imu[a + g].z)));
                }
            },
        }
        Ok(())
    }
}

fn main() {
    let inname = nri::parse_in_arg(&mut env::args().skip(1));

    *OUTPUT_MODE.write().unwrap() = FT;
    let mut header = String::from("Timestamp, Teensy dt, Packet number");
    for i in 0..30 {
        header.push_str(&format!(", FT{}", i));
    }
    nri::do_binary::<Packet>(&header,
                                (inname.clone(), Some(Path::new(&inname).with_extension("ft.csv").to_str().unwrap().to_string())));

    *OUTPUT_MODE.write().unwrap() = ACC;
    nri::do_binary::<Packet>("Timestamp, FIFO position, Acc X, Acc Y, Acc Z",
                                (inname.clone(), Some(Path::new(&inname).with_extension("acc.csv").to_str().unwrap().to_string())));
    *OUTPUT_MODE.write().unwrap() = GYRO;
    nri::do_binary::<Packet>("Timestamp, FIFO position, Gyro X, Gyro Y, Gyro Z",
                                (inname.clone(), Some(Path::new(&inname).with_extension("gyro.csv").to_str().unwrap().to_string())));
    *OUTPUT_MODE.write().unwrap() = MAG;
    nri::do_binary::<Packet>("Timestamp, Mag X, Mag Y, Mag Z",
                                (inname.clone(), Some(Path::new(&inname).with_extension("mag.csv").to_str().unwrap().to_string())));
}


