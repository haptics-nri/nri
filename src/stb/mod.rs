//! Service to read data from the STB and attached sensors

/// Which end effector is in use (i.e. not parked)
pub enum ParkState {
    /// All end effectors parked
    None = 0,
    /// The Biotac is out
    BioTac = 1,
    /// The Optoforce is out
    OptoForce = 2,
    /// The rigid stick is out
    Stick = 4,
    /// Multiple end effectors unparked! The sky is falling!
    Multiple = -1
}

group_attr!{
    #[cfg(target_os = "linux")]

    extern crate serial;
    extern crate time;
    use ::comms::{Controllable, CmdFrom, Block};
    use std::io::{Read, Write};
    use std::fs::File;
    use std::sync::mpsc::Sender;
    use std::{u8, ptr, mem, slice, ops};
    use std::ops::DerefMut;
    use std::fmt::{self, Display, Debug, Formatter};
    use self::serial::prelude::*;


    pub struct STB {
        port: Box<serial::SerialPort>,
        file: File,
        i: usize,
        start: time::Tm,
    }

    #[repr(packed)]
    struct XYZ<T> {
        x: T,
        y: T,
        z: T
    }
    #[repr(packed)]
    struct Packet {
        ft     : [u8; 30],
        count  : u8,
        n_acc  : u8,
        n_gyro : u8,
        imu    : [XYZ<i16>]
    }

    impl Packet {
        unsafe fn new(buf: &[u8]) -> Result<Packet, String> {
            unsafe fn checksum(buf: &[u8]) -> Result<(), String> {
                let sum: u8 = buf[..buf.len()-1].into_iter().fold(0, |a: u8, b: &u8| -> u8 { u8::wrapping_add(a, *b) });
                match buf[buf.len()-1] {
                    s if s == sum => Ok(()),
                    s => Err(format!("Received STB packet with wrong checksum (it says {}, I calculate {})!", s, sum)),
                }
            }

            unsafe fn only_stb(buf: &[u8]) -> Packet {
                let mut p: Packet = Packet {
                    ft     : mem::zeroed(),
                    count  : buf[0],
                    n_acc  : 0,
                    n_gyro : 0,
                    imu    : []
                };
                for i in 0..30 { p.ft[i] = buf[1 + i]; }
                p
            }

            unsafe fn imu_and_stb(buf: &[u8], a: usize, g: usize) -> Packet {
                let s = 2 + 6*(a + g + 1);
                let mut p: Packet = Packet {
                    ft     : mem::zeroed(),
                    count  : buf[s],
                    n_acc  : a as u8,
                    n_gyro : g as u8,
                    imu    : mem::zeroed::<[XYZ<i16>; a+g+1]>()
                };
                for i in 0..30 { p.ft[i] = buf[s+1 + i]; }
                ptr::copy::<XYZ<i16>>(buf[2..s].as_ptr() as *const XYZ<i16>, p.imu.as_mut_ptr(), (a+g+1) as usize);
                p
            }

            match buf.len() {
                x if x < 31 => Err(format!("Implausibly small packet ({}) from STB!", x)),
                31 => Ok(only_stb(buf)),
                32 => {
                    try!(checksum(buf));
                    Ok(only_stb(buf))
                },
                x => {
                    let a = buf[0] as usize;
                    let g = buf[1] as usize;
                    match x {
                        t if t == 31 + 2 + 6*(a + g + 1) => Ok(imu_and_stb(buf, a, g)),
                        t if t == 31 + 2 + 6*(a + g + 1) + 1 => {
                            try!(checksum(buf));
                            Ok(imu_and_stb(buf, a, g))
                        },
                        _ => Err(format!("Impossible packet size ({}) from STB!", x)),
                    }
                },
            }
        }
    }

    impl<T: Display> Debug for XYZ<T> {
        fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
            try!(write!(f, "({:#6}, {:#6}, {:#6})", self.x, self.y, self.z));
            Ok(())
        }
    }

    impl Debug for Packet {
        fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
            try!(write!(f, "IMU ({} acc, {} gyro, {} mag)", self.n_acc, self.n_gyro, self.n_acc + self.n_gyro > 0));
            try!(write!(f, "\t"));
            try!(write!(f, "ft sum={}", self.ft.iter().fold(0, ops::Add::add)));
            try!(write!(f, "\t"));
            try!(write!(f, "count={}", self.count));
            Ok(())
        }
    }

    guilty! {
        impl Controllable for STB {
            const NAME: &'static str = "stb",
            const BLOCK: Block = Block::Immediate,

            fn setup(_: Sender<CmdFrom>, _: Option<String>) -> STB {
                assert_eq!(mem::size_of::<Packet>(), u8::MAX as usize);

                let mut port = serial::open("/dev/ttySTB").unwrap();
                port.reconfigure(&|settings| {
                    try!(settings.set_baud_rate(serial::Baud115200));
                    Ok(())
                }).unwrap();
                port.write(&['1' as u8]);

                STB { port: Box::new(port), file: File::create("data/stb.dat").unwrap(), i: 0, start: time::now() }
            }

            fn step(&mut self, _: Option<String>) {
                self.i += 1;

                let mut size_buf = [0u8; 4];
                let packet_size = match self.port.read(&mut size_buf) {
                    Ok(l) if l == size_buf.len() => {
                        if &size_buf[..3] == b"aaa" {
                            size_buf[3] as usize
                        } else {
                            errorln!("The STB did not sent the expected packet size prefix!");
                            return;
                        }
                    },
                    Ok(_) => {
                        errorln!("Something went wrong reading the packet size from the STB!");
                        return;
                    },
                    Err(e) => {
                        errorln!("Error reading packet size from the STB: {:?}", e);
                        return;
                    }
                };
                let mut buf = Vec::<u8>::with_capacity(packet_size);
                match self.port.deref_mut().take(packet_size as u64).read_to_end(&mut buf) {
                    Ok(n) if n == packet_size => {
                        let packet = match unsafe { Packet::new(&buf) } {
                            Ok(p) => p,
                            Err(s) => {
                                errorln!("{}", s);
                                return;
                            },
                        };
                        self.file.write_all(unsafe { slice::from_raw_parts(&time::get_time() as *const _ as *const _, mem::size_of::<time::Timespec>()) });
                        self.file.write_all(unsafe { slice::from_raw_parts(&packet as *const _ as *const _, mem::size_of_val(&packet)) });
                    },
                    Ok(_)   => errorln!("Read partial packet from STB?"),
                    Err(e)  => errorln!("Error reading packet from STB: {:?}", e)
                }
            }

            fn teardown(&mut self) {
                self.port.write(&['2' as u8]);
                let end = time::now();
                let millis = (end - self.start).num_milliseconds() as f64;
                println!("{} STB packets grabbed in {} s ({} FPS)!", self.i, millis/1000.0, 1000.0*(self.i as f64)/millis);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
stub!(STB);

