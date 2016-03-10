//! Service to read data from the Teensy and attached sensors

#![cfg_attr(feature = "nightly", feature(read_exact))]

#[macro_use] extern crate utils;
#[macro_use] extern crate comms;

#[macro_use] extern crate guilt_by_association;
#[macro_use] extern crate custom_derive;
#[macro_use] extern crate conv;

custom_derive! {
    /// Which end effector is in use (i.e. not parked)
    #[derive(Copy, Clone, Eq, PartialEq, Debug, TryFrom(u8))]
    pub enum ParkState {
        /// All end effectors parked
        None = 0,
        /// The Optoforce is out
        OptoForce = 1,
        /// The rigid stick is out
        Stick = 2,
        /// The Biotac is out
        BioTac = 4,
        /// Multiple end effectors unparked! The sky is falling!
        Multiple = -1
    }
}

impl ParkState {
    pub fn short(&self) -> &str {
        match *self {
            None      => "center", // HACK
            OptoForce => "opto",
            Stick     => "stick",
            BioTac    => "bio",
            Multiple  => "multi"
        }
    }
}

#[cfg(not(target_os = "linux"))]
impl ParkState {
    pub fn metermaid() -> Option<ParkState> {
        Some(ParkState::Stick)
    }
}

group_attr!{
    #[cfg(target_os = "linux")]

    extern crate scribe;

    extern crate serial;
    extern crate time;

    use comms::{Controllable, CmdFrom, Block};
    use scribe::{Writer, Writable};
    use std::io::{self, Read, Write};
    use std::fs::File;
    use std::sync::mpsc::Sender;
    use std::{u8, ptr, mem, ops};
    use std::fmt::{self, Display, Debug, Formatter};
    use std::time::Duration;
    use serial::prelude::*;
    use conv::TryFrom;

    trait RFC980: Read {
        fn read_exact_shim(&mut self, buf: &mut [u8]) -> io::Result<()> {
            self.read_exact_real(buf)
        }

        #[cfg(feature = "nightly")]
        fn read_exact_real(&mut self, buf: &mut [u8]) -> io::Result<()> {
            self.read_exact(buf)
        }

        #[cfg(not(feature = "nightly"))]
        fn read_exact_real(&mut self, mut buf: &mut [u8]) -> io::Result<()> {
            while !buf.is_empty() {
                match self.read(buf) {
                    Ok(0)   => break,
                    Ok(n)   => {
                        let tmp = buf;
                        buf = &mut tmp[n..];
                    },
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted
                            => {},
                    Err(e)  => return Err(e),
                }
            }

            if !buf.is_empty() {
                Err(io::Error::new(io::ErrorKind::Other, "failed to fill whole buffer"))
            } else {
                Ok(())
            }
        }
    }

    impl<T: Read> RFC980 for T {}

    trait Coffee: Read + Write {
        fn coffee<W: Write>(self, w: W) -> CoffeeImpl<Self, W> where Self: Sized {
            CoffeeImpl { parent: self, writer: w }
        }
    }

    struct CoffeeImpl<RW: Read + Write, W: Write> {
        parent: RW,
        writer: W,
    }

    impl<RW: Read + Write, W: Write> Read for CoffeeImpl<RW, W> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = try!(self.parent.read(buf));
            try!(self.writer.write_all(&buf[..n]));
            Ok(n)
        }
    }

    impl<RW: Read + Write, W: Write> Write for CoffeeImpl<RW, W> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.parent.write(buf) }
        fn flush(&mut self)             -> io::Result<()>    { self.parent.flush()    }
    }

    impl<T: Read + Write> Coffee for T {}

    trait StaticReadWrite: Read + Write + 'static {}
    impl<T: Read + Write + 'static> StaticReadWrite for T {}

    fn byte_copy(from: &[u8], mut to: &mut [u8]) -> usize {
        to.write(from).unwrap()
    }

    fn serialport() -> Box<StaticReadWrite> {
        let mut port = serial::open("/dev/ttyTEENSY").unwrap();
        port.reconfigure(&|settings| {
            try!(settings.set_baud_rate(serial::Baud115200));
            Ok(())
        }).unwrap();
        port.set_timeout(Duration::from_millis(100)).unwrap();
        if false {
            Box::new(port.coffee(File::create("teensydump.dat").unwrap()))
        } else {
            Box::new(port)
        }
    }

    impl ParkState {
        pub fn metermaid() -> Option<ParkState> {
            let mut port = serialport();

            port.write_all(&['4' as u8]).unwrap();

            let mut buf = [0u8; 1];
            match port.read_exact_shim(&mut buf) {
                Ok(())         => {
                    match ParkState::try_from(!(buf[0] | 0b1111_1000)) {
                        Ok(ps) => Some(ps),
                        Err(_) => Some(ParkState::Multiple)
                    }
                },
                Err(..)         => {
                    None
                }
            }
        }
    }

    pub struct Teensy {
        port: Box<StaticReadWrite>,
        file: Writer<Packet>,
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
    #[allow(dead_code)]
    struct Packet {
        stamp  : time::Timespec,
        ft     : [u8; 31],
        n_acc  : u8,
        n_gyro : u8,
        imu    : [XYZ<i16>; 37]
    }

    unsafe impl Writable for Packet {}

    impl Packet {
        unsafe fn new(buf: &[u8]) -> Result<Packet, String> {
            fn checksum(buf: &[u8]) -> Result<(), String> {
                let sum = buf[..buf.len()-1].into_iter().fold(0, |a, b| { u8::wrapping_add(a, *b) });
                match buf[buf.len()-1] {
                    s if s == sum => Ok(()),
                    s => Err(format!("Received Teensy packet with wrong checksum (it says {}, I calculate {})!", s, sum)),
                }
            }

            unsafe fn only_analog(buf: &[u8]) -> Packet {
                let mut p: Packet = Packet {
                    stamp  : time::get_time(),
                    ft     : mem::zeroed::<[u8; 31]>(),
                    n_acc  : 0,
                    n_gyro : 0,
                    imu    : mem::zeroed::<[XYZ<i16>; 37]>(),
                };
                byte_copy(buf, &mut p.ft);
                p
            }

            unsafe fn imu_and_analog(buf: &[u8], a: usize, g: usize) -> Packet {
                let s = 2 + 6*(a + g + 1);
                let mut p: Packet = Packet {
                    stamp  : time::get_time(),
                    ft     : mem::zeroed::<[u8; 31]>(),
                    n_acc  : a as u8,
                    n_gyro : g as u8,
                    imu    : mem::zeroed::<[XYZ<i16>; 37]>()
                };
                byte_copy(&buf[s..], &mut p.ft);
                ptr::copy::<XYZ<i16>>(buf[2..s].as_ptr() as *const XYZ<i16>, p.imu.as_mut_ptr(), (a+g+1) as usize);
                p
            }

            match buf.len() {
                x if x < 31 => Err(format!("Implausibly small packet ({}) from Teensy!", x)),
                31 => Ok(only_analog(buf)),
                32 => {
                    try!(checksum(buf));
                    Ok(only_analog(buf))
                },
                x => {
                    let a = buf[0] as usize;
                    let g = buf[1] as usize;
                    match x {
                        t if t == 31 + 2 + 6*(a + g + 1) => Ok(imu_and_analog(buf, a, g)),
                        t if t == 31 + 2 + 6*(a + g + 1) + 1 => {
                            try!(checksum(buf));
                            Ok(imu_and_analog(buf, a, g))
                        },
                        _ => Err(format!("Impossible packet size ({}) from Teensy!", x)),
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
            Ok(())
        }
    }

    guilty! {
        impl Controllable for Teensy {
            const NAME: &'static str = "teensy",
            const BLOCK: Block = Block::Period(333_333),

            fn setup(_: Sender<CmdFrom>, _: Option<String>) -> Teensy {
                assert_eq!(mem::size_of::<Packet>(), u8::MAX as usize + mem::size_of::<time::Timespec>());

                let mut port = serialport();
                port.write_all(&['1' as u8]).unwrap();

                Teensy { port: port, file: Writer::with_file("teensy.dat"), i: 0, start: time::now() }
            }

            fn step(&mut self, _: Option<String>) {
                self.i += 1;

                /*
                let mut b = [0u8; 4096];
                self.port.read_exact_shim(&mut b).err().map(|e| println!("Teensy read error {:?}", e));
                */
                let mut size_buf = [0u8; 4];
                let packet_size = match self.port.read_exact_shim(&mut size_buf) {
                    Ok(()) => {
                        if &size_buf[..3] == b"aaa" {
                            size_buf[3] as usize
                        } else {
                            errorln!("The Teensy did not send the expected packet size prefix! instead {:?}", size_buf);
                            let mut scanning = [0u8; 1];
                            let mut count = 0;
                            while count < 3 {
                                self.port.read_exact_shim(&mut scanning).unwrap();
                                if scanning[0] == 'a' as u8 {
                                    count += 1;
                                } else {
                                    count = 0;
                                }
                            }
                            self.port.read_exact_shim(&mut scanning).unwrap();
                            scanning[0] as usize
                        }
                    },
                    Err(e) => {
                        errorln!("Error reading packet size from the Teensy: {:?}", e);
                        return;
                    }
                };
                let mut buf = vec![0u8; packet_size];
                match self.port.read_exact_shim(&mut buf[..]) {
                    Ok(()) => {
                        let packet = match unsafe { Packet::new(&buf) } {
                            Ok(p) => p,
                            Err(s) => {
                                errorln!("{}", s);
                                return;
                            },
                        };
                        self.file.write(packet);
                    },
                    Err(e)  => errorln!("Error reading packet from Teensy: {:?}", e)
                }
            }

            fn teardown(&mut self) {
                self.port.write_all(&['2' as u8]).unwrap();
                let end = time::now();
                let millis = (end - self.start).num_milliseconds() as f64;
                println!("{} Teensy packets grabbed in {} s ({} FPS)!", self.i, millis/1000.0, 1000.0*(self.i as f64)/millis);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
stub!(Teensy);
