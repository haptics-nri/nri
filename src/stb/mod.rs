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
    use std::{mem, slice, ops};
    use std::fmt::{self, Display, Debug, Formatter};
    use self::serial::prelude::*;


    pub struct STB {
        port: Box<serial::SerialPort>,
        file: File
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
        imu    : [XYZ<i16>; 37] // pad out struct to 255 bytes
    }

    impl Packet {
        unsafe fn new(buf: &[u8]) -> Result<Packet, &str> {
            fn checksum(buf: &[u8]) -> Result<(), &str> {
                let sum: u8 = buf[..buf.len()-1].into_iter().fold(0, u8::wrapping_add);
                match sum {
                    buf[buf.len()-1] => Ok(()),
                    s => Err(&format!("Received STB packet with wrong checksum (it says {}, I calculate {})!", s, sum)),
                }
            }

            fn only_stb(buf: &[u8]) -> Packet {
                Packet {
                    ft     : buf[1..30],
                    count  : buf[30],
                    n_acc  : 0,
                    n_gyro : 0,
                    imu    : mem::zeroed()
                },
            }

            fn imu_and_stb(buf: &[u8], a: u8, g: u8) -> Packet {
                let s: usize = 2 + 6*(a + g + 1);
                let p = Packet {
                    ft     : buf[s..s+29],
                    count  : buf[s+29],
                    n_acc  : a,
                    n_gyro : g,
                    imu    : mem::zeroed()
                };
                p.imu[..a+g+1] = slice::<XYZ<i16>>::from_raw_parts(&buf[..s], a+g+1);
                p
            }

            match buf.len() {
                x if x < 31 => Err(&format!("Implausibly small packet ({}) from STB!", x)),
                31 => Some(only_stb(buf)),
                32 => {
                    try!(checksum(buf));
                    Some(only_stb(buf))
                },
                x => {
                    let a = buf[1];
                    let g = buf[2];
                    match x {
                        31 + 2 + 6*(a + g + 1) => Some(imu_and_stb(buf, a, g)),
                        31 + 2 + 6*(a + g + 1) + 1 => {
                            try!(checksum(buf));
                            Some(imu_and_stb(buf, a, g))
                        },
                        _ => Err(&format("Impossible packet size ({}) from STB!", x)),
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
            try!(write!(f, "accel {:?}", self.accel));
            try!(write!(f, "\t"));
            try!(write!(f, "gyro {:?}", self.gyro));
            try!(write!(f, "\t"));
            try!(write!(f, "mag {:?}", self.mag));
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
                assert_eq!(mem::size_of::<Packet>(), LEN);

                let mut port = serial::open("/dev/ttySTB").unwrap();
                port.reconfigure(&|settings| {
                    try!(settings.set_baud_rate(serial::Baud115200));
                    Ok(())
                }).unwrap();
                port.write(&['1' as u8]);

                STB { port: Box::new(port), file: File::create("data/stb.dat").unwrap() }
            }

            fn step(&mut self, _: Option<String>) {
                let mut size_buf = [u8; 4];
                let packet_size = match self.port.read(&mut size_buf) {
                    Ok(size_buf.len()) => {
                        if size_buf[..3] == "aaa" {
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
                }
                let mut buf = Vec<u8>::with_capacity(packet_size);
                match self.port.by_ref().take(packet_size).read_to_end(&mut buf) {
                    Ok(packet_size) => {
                        let packet = match unsafe { Packet::new(&buf, n_acc, n_gyro) } {
                            Ok(p) => p,
                            Err(s) => {
                                errorln!("{}", s);
                                return;
                            },
                        };
                        self.file.write_all(unsafe { slice::from_raw_parts(&time::get_time() as *const _ as *const _, mem::size_of::<time::Timespec>()) });
                        self.file.write_all(&packet);
                    },
                    Ok(_)   => errorln!("Read partial packet from STB?"),
                    Err(e)  => errorln!("Error reading packet from STB: {:?}", e)
                }
            }

            fn teardown(&mut self) {
                self.port.write(&['2' as u8]);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
stub!(STB);

