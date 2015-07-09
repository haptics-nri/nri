//! Service to read data from the STB and attached sensors

group_attr!{
    #[cfg(target_os = "linux")]

    extern crate serial;
    use ::comms::{Controllable, CmdFrom};
    use std::io::{Read, Write};
    use std::sync::mpsc::Sender;
    use std::mem;
    use std::fmt::{self, Display, Debug, Formatter};
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
        accel: XYZ<u16>,
        gyro:  XYZ<u16>,
        mag:   XYZ<u16>,
        ft:    [u8; 30],
        count: u8
    }
    const LEN: usize = 49;

    impl Packet {
        unsafe fn new(buf: &[u8; LEN]) -> Packet {
            mem::transmute(*buf)
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
            // skip F/T for now
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

                let mut port = serial::open("/dev/ttyACM1").unwrap();
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
                        let packet = unsafe { Packet::new(&buf) };
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
                self.port.write(&[0]);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
stub!(STB);

