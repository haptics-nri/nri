extern crate time;

#[macro_use] extern crate nri;

use std::{env, mem, ptr, slice};
use std::io::{self, Read};
use std::fs::File;

trait RFC980: Read {
    fn read_exact(&mut self, mut buf: &mut [u8]) -> io::Result<()> {
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

#[allow(unused)]
#[repr(packed)]
struct XYZ<T> {
    x: T,
    y: T,
    z: T
}

#[allow(unused)]
#[repr(packed)]
struct RawPacket {
    ft     : [u8; 31],
    n_acc  : u8,
    n_gyro : u8,
    imu    : [XYZ<i16>; 37]
}

impl RawPacket {
    unsafe fn new(buf: &[u8]) -> Result<RawPacket, String> {
        fn checksum(buf: &[u8]) -> Result<(), String> {
            let sum = buf[..buf.len()-1].into_iter().fold(0, |a, b| { u8::wrapping_add(a, *b) });
            match buf[buf.len()-1] {
                s if s == sum => Ok(()),
                s => Err(format!("Received STB packet with wrong checksum (it says {}, I calculate {})!", s, sum)),
            }
        }

        unsafe fn only_stb(buf: &[u8]) -> RawPacket {
            let mut p: RawPacket = RawPacket {
                ft     : mem::zeroed::<[u8; 31]>(),
                n_acc  : 0,
                n_gyro : 0,
                imu    : mem::zeroed::<[XYZ<i16>; 37]>(),
            };
            for i in 0..31 { p.ft[i] = buf[i]; }
            p
        }

        unsafe fn imu_and_stb(buf: &[u8], a: usize, g: usize) -> RawPacket {
            let s = 2 + 6*(a + g + 1);
            let mut p: RawPacket = RawPacket {
                ft     : mem::zeroed::<[u8; 31]>(),
                n_acc  : a as u8,
                n_gyro : g as u8,
                imu    : mem::zeroed::<[XYZ<i16>; 37]>()
            };
            for i in 0..31 { p.ft[i] = buf[s + i]; }
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

fn go<R: io::Read, W: io::Write>(mut reader: R, mut writer: W) -> Result<(),io::Error> {
    loop {
        let mut size_buf = [0u8; 4];
        let packet_size = match reader.read_exact(&mut size_buf) {
            Ok(()) => {
                if &size_buf[..3] == b"aaa" {
                    size_buf[3] as usize
                } else {
                    errorln!("The STB did not send the expected packet size prefix! instead {:?}", size_buf);
                    let mut scanning = [0u8; 1];
                    let mut count = 0;
                    while count < 3 {
                        reader.read_exact(&mut scanning).unwrap();
                        if scanning[0] == b'a' {
                            count += 1;
                        } else {
                            count = 0;
                        }
                    }
                    reader.read_exact(&mut scanning).unwrap();
                    scanning[0] as usize
                }
            },
            Err(e) => {
                return Err(e);
            }
        };
        let mut buf = vec![0u8; packet_size];
        match reader.read_exact(&mut buf[..]) {
            Ok(()) => {
                match unsafe { RawPacket::new(&buf) } {
                    Ok(p) => {
                        writer.write_all(unsafe { slice::from_raw_parts(&time::get_time() as *const _ as *const _, mem::size_of::<time::Timespec>()) }).unwrap();
                        writer.write_all(unsafe { slice::from_raw_parts(&p as *const _ as *const _, mem::size_of_val(&p)) }).unwrap();
                    },
                    Err(s) => errorln!("{:?}", s)
                }
            },
            Err(e)  => return Err(e)
        }
    }
}

fn main() {
    let (inname, outname) = nri::parse_inout_args(&mut env::args());
    println!("{:?}", go(File::open(&inname).unwrap(), File::create(&outname).unwrap()));
}


