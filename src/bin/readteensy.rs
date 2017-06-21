extern crate time;
#[macro_use] extern crate closet;
extern crate spawner;

extern crate nri;

use spawner::Spawner;
use std::cell::Cell;
use std::sync::Arc;
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

thread_local! {
    static OUTPUT_MODE: Cell<OutputMode> = Cell::new(FT);
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

        match OUTPUT_MODE.with(|om| om.get()) {
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

    let bars = Arc::new(nri::MultiProgress::new());
    let mut spawner = Spawner::new();

    spawner.spawn_collected(clone_army!([bars, inname] move || {
        OUTPUT_MODE.with(|om| om.set(FT));
        let mut header = String::from("Timestamp, Teensy dt, Packet number");
        for i in 0..30 {
            header.push_str(&format!(", FT{}", i));
        }
        nri::do_binary::<Packet>(&header, nri::Bar::Multi("FT", bars),
                                 (inname.clone(), Some(Path::new(&inname).with_extension("ft.csv").to_str().unwrap().to_string())));
    }));

    spawner.spawn_collected(clone_army!([bars, inname] move || {
        OUTPUT_MODE.with(|om| om.set(ACC));
        nri::do_binary::<Packet>("Timestamp, FIFO position, Acc X, Acc Y, Acc Z", nri::Bar::Multi("Acc", bars),
                                 (inname.clone(), Some(Path::new(&inname).with_extension("acc.csv").to_str().unwrap().to_string())));
    }));

    spawner.spawn_collected(clone_army!([bars, inname] move || {
        OUTPUT_MODE.with(|om| om.set(GYRO));
        nri::do_binary::<Packet>("Timestamp, FIFO position, Gyro X, Gyro Y, Gyro Z", nri::Bar::Multi("Gyro", bars),
                                 (inname.clone(), Some(Path::new(&inname).with_extension("gyro.csv").to_str().unwrap().to_string())));
    }));

    spawner.spawn_collected(clone_army!([bars, inname] move || {
        OUTPUT_MODE.with(|om| om.set(MAG));
        nri::do_binary::<Packet>("Timestamp, Mag X, Mag Y, Mag Z", nri::Bar::Multi("Mag", bars),
                                 (inname.clone(), Some(Path::new(&inname).with_extension("mag.csv").to_str().unwrap().to_string())));
    }));

    drop(spawner); // join all threads
    bars.join_and_clear().unwrap();
}

