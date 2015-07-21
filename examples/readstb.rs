#[macro_use] extern crate lazy_static;
extern crate time;

#[macro_use] mod common;

use std::fmt;

#[repr(packed)]
struct XYZ<T> {
    x: T,
    y: T,
    z: T
}
#[repr(packed)]
struct Packet {
    stamp:  time::Timespec,
    accel:  XYZ<i16>,
    gyro:   XYZ<i16>,
    mag:    XYZ<i16>,
    _ft:    [u8; 30],
    _count: u8,
    _zero:  u8,
}

impl fmt::Debug for Packet {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        try!(write!(f, "{:.9}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                    self.stamp.sec as f64 + self.stamp.nsec as f64 / 1_000_000_000f64,
                    self.accel.x, self.accel.y, self.accel.z,
                    self.gyro.x, self.gyro.y, self.gyro.z,
                    self.mag.x.to_be(), self.mag.y.to_be(), self.mag.z.to_be()));
        Ok(())
    }
}

fn main() {
    common::read_binary::<Packet>("Timestamp, AccX, AccY, AccZ, GyroX, GyroY, GyroZ, MagX, MagY, MagZ");
}


