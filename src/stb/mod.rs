//! Service to read data from the STB and attached sensors

macro_rules! swap {
    ($a:expr, $b:expr) => {{
        let tmp = $a;
        $a = $b;
        $b = tmp;
    }}
}

group_attr!{
    #[cfg(target_os = "linux")]

    extern crate serial;
    use ::comms::{Controllable, CmdFrom};
    use std::io::{Read, Write};
    use std::sync::mpsc::Sender;
    use std::mem;
    use std::fmt::{self, Display, Debug, Formatter};
    use std::ops;
    use self::serial::prelude::*;


    pub struct STB {
        port: Box<serial::SerialPort>
    }

    #[repr(packed)]
    struct XYZ<T> {
        x: T,
        y: T,
        z: T
    }
    #[repr(packed)]
    struct Packet {
        accel: XYZ<i16>,
        gyro:  XYZ<i16>,
        mag:   XYZ<i16>,
        ft:    [u8; 30],
        count: u8
    }
    const LEN: usize = 49;

    impl Packet {
        unsafe fn new(mut buf: [u8; LEN]) -> Packet {
            for i in 0..6 {
                swap!(buf[2*i], buf[2*i+1]);
            }
            mem::transmute(buf)
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
            try!(write!(f, "\n"));
            Ok(())
        }
    }

    guilty! {
        impl Controllable for STB {
            const NAME: &'static str = "stb",

            fn setup(_: Sender<CmdFrom>, _: Option<String>) -> STB {
                assert_eq!(mem::size_of::<Packet>(), LEN);

                let mut port = serial::open("/dev/ttySTB").unwrap();
                port.reconfigure(&|settings| {
                    try!(settings.set_baud_rate(serial::Baud115200));
                    Ok(())
                }).unwrap();
                port.write(&[1]);

                STB { port: Box::new(port) }
            }

            fn step(&mut self, _: Option<String>) -> bool {
                let mut buf = [0; LEN];
                match self.port.read(&mut buf) {
                    Ok(LEN) => {
                        let packet = unsafe { Packet::new(buf) };
                        if packet.count == 0 {
                            println!("Read full packet from STB: {:?}", packet);
                        }
                    },
                    Ok(_)   => println!("Read partial packet from STB?"),
                    Err(e)  => errorln!("Error reading from STB: {:?}", e)
                }

                false
            }

            fn teardown(&mut self) {
                self.port.write(&[2]);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
stub!(STB);

