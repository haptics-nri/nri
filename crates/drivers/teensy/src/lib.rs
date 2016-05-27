//! Service to read data from the Teensy and attached sensors

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
            ParkState::None      => "center", // HACK
            ParkState::OptoForce => "opto",
            ParkState::Stick     => "stick",
            ParkState::BioTac    => "bio",
            ParkState::Multiple  => "multi"
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
    extern crate rustc_serialize as serialize;
    extern crate gnuplot;

    use comms::{Controllable, CmdFrom, Block, RestartableThread};
    use scribe::{Writer, Writable};
    use std::io::{self, Read, Write};
    use std::fs::File;
    use std::thread;
    use std::sync::mpsc::Sender;
    use std::{u8, ptr, mem, ops};
    use std::fmt::{self, Display, Debug, Formatter};
    use std::time::Duration;
    use serial::prelude::*;
    use conv::TryFrom;
    use serialize::base64::{self, ToBase64};
    use gnuplot::{Figure, PlotOption, Coordinate, AxesCommon};

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
            match port.read_exact(&mut buf) {
                Ok(())         => {
                    match ParkState::try_from(!(buf[0] | 0b1111_1000)) {
                        Ok(ps) => Some(ps),
                        Err(_) => Some(ParkState::Multiple)
                    }
                },
                Err(..)        => {
                    None
                }
            }
        }
    }

    type PngStuff = (Sender<CmdFrom>, Vec<Packet>);

    pub struct Teensy {
        port: Box<StaticReadWrite>,
        file: Writer<Packet>,
        i: usize,
        buf: Option<Vec<Packet>>,
        tx: Sender<CmdFrom>,
        png: RestartableThread<PngStuff>,
        start: time::Tm,
    }

    #[repr(packed)]
    #[derive(Copy, Clone)]
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
    impl Copy for Packet {}
    impl Clone for Packet { fn clone(&self) -> Packet { *self } }

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

            fn setup(tx: Sender<CmdFrom>, _: Option<String>) -> Teensy {
                assert_eq!(mem::size_of::<Packet>(), u8::MAX as usize + mem::size_of::<time::Timespec>());

                let mut port = serialport();
                port.write_all(&['1' as u8]).unwrap();

                // some stuff for the RestartableThread
                let mut idx = 0;
                let mut fig = Figure::new();
                let mut data = Vec::with_capacity(10240);
                let start = time::get_time();

                Teensy {
                    port: port,
                    file: Writer::with_file("teensy.dat"),
                    i: 0,
                    start: time::now(),
                    tx: tx,
                    buf: None,
                    png: RestartableThread::new("Teensy PNG thread", move |(sender, vec): PngStuff| {
                        // process data
                        let mut t  = [0.0; 3000];
                        let mut fx = [0.0; 3000];
                        let mut fy = [0.0; 3000];
                        let mut fz = [0.0; 3000];
                        let mut tx = [0.0; 3000];
                        let mut ty = [0.0; 3000];
                        let mut tz = [0.0; 3000];
                        for i in 0..3000 {
                            let diff = (vec[i].stamp - start).to_std().unwrap();
                            t[i] = diff.as_secs() as f64 + (diff.subsec_nanos() as f64 / 1.0e9);
                            let mut ft = [(((vec[i].ft[0]  as u32) << 8) + (vec[i].ft[1]  as u32)) as i32,
                                          (((vec[i].ft[2]  as u32) << 8) + (vec[i].ft[3]  as u32)) as i32,
                                          (((vec[i].ft[4]  as u32) << 8) + (vec[i].ft[5]  as u32)) as i32,
                                          (((vec[i].ft[6]  as u32) << 8) + (vec[i].ft[7]  as u32)) as i32,
                                          (((vec[i].ft[8]  as u32) << 8) + (vec[i].ft[9]  as u32)) as i32,
                                          (((vec[i].ft[10] as u32) << 8) + (vec[i].ft[11] as u32)) as i32];
                            for j in 0..6 {
                                if ft[j] >= 2048 {
                                    ft[j] -= 4096;
                                }
                            }
                            const BIAS: [f64; 6] = [-0.1884383674, 0.2850118688, -0.180718143, -0.191009933, 0.3639300747, -0.4307167708];
                            const TF: [[f64; 6]; 6] = [[0.00679, 0.01658, -0.04923, 6.20566, 0.15882, -6.19201],
                                                       [0.11638, -7.31729, -0.04322, 3.54949, -0.08024, 3.57115],
                                                       [10.35231, 0.32653, 10.61091, 0.29668, 10.33382, 0.25761],
                                                       [0.00022, -0.0414, 0.14917, 0.02435, -0.15234, 0.01567],
                                                       [-0.16837, -0.00464, 0.08561, -0.03311, 0.08763, 0.03721],
                                                       [0.00128, -0.08962, 0.00085, -0.08785, 0.00204, -0.0879]];
                            fx[i] = (TF[0][0] * (((ft[0] as f64) * 0.002) - BIAS[0]))
                                  + (TF[0][1] * (((ft[1] as f64) * 0.002) - BIAS[1]))
                                  + (TF[0][2] * (((ft[2] as f64) * 0.002) - BIAS[2]))
                                  + (TF[0][3] * (((ft[3] as f64) * 0.002) - BIAS[3]))
                                  + (TF[0][4] * (((ft[4] as f64) * 0.002) - BIAS[4]))
                                  + (TF[0][5] * (((ft[5] as f64) * 0.002) - BIAS[5]));
                            fy[i] = (TF[1][0] * (((ft[0] as f64) * 0.002) - BIAS[0]))
                                  + (TF[1][1] * (((ft[1] as f64) * 0.002) - BIAS[1]))
                                  + (TF[1][2] * (((ft[2] as f64) * 0.002) - BIAS[2]))
                                  + (TF[1][3] * (((ft[3] as f64) * 0.002) - BIAS[3]))
                                  + (TF[1][4] * (((ft[4] as f64) * 0.002) - BIAS[4]))
                                  + (TF[1][5] * (((ft[5] as f64) * 0.002) - BIAS[5]));
                            fz[i] = (TF[2][0] * (((ft[0] as f64) * 0.002) - BIAS[0]))
                                  + (TF[2][1] * (((ft[1] as f64) * 0.002) - BIAS[1]))
                                  + (TF[2][2] * (((ft[2] as f64) * 0.002) - BIAS[2]))
                                  + (TF[2][3] * (((ft[3] as f64) * 0.002) - BIAS[3]))
                                  + (TF[2][4] * (((ft[4] as f64) * 0.002) - BIAS[4]))
                                  + (TF[2][5] * (((ft[5] as f64) * 0.002) - BIAS[5]));
                            tx[i] = (TF[3][0] * (((ft[0] as f64) * 0.002) - BIAS[0]))
                                  + (TF[3][1] * (((ft[1] as f64) * 0.002) - BIAS[1]))
                                  + (TF[3][2] * (((ft[2] as f64) * 0.002) - BIAS[2]))
                                  + (TF[3][3] * (((ft[3] as f64) * 0.002) - BIAS[3]))
                                  + (TF[3][4] * (((ft[4] as f64) * 0.002) - BIAS[4]))
                                  + (TF[3][5] * (((ft[5] as f64) * 0.002) - BIAS[5]));
                            ty[i] = (TF[4][0] * (((ft[0] as f64) * 0.002) - BIAS[0]))
                                  + (TF[4][1] * (((ft[1] as f64) * 0.002) - BIAS[1]))
                                  + (TF[4][2] * (((ft[2] as f64) * 0.002) - BIAS[2]))
                                  + (TF[4][3] * (((ft[3] as f64) * 0.002) - BIAS[3]))
                                  + (TF[4][4] * (((ft[4] as f64) * 0.002) - BIAS[4]))
                                  + (TF[4][5] * (((ft[5] as f64) * 0.002) - BIAS[5]));
                            tz[i] = (TF[5][0] * (((ft[0] as f64) * 0.002) - BIAS[0]))
                                  + (TF[5][1] * (((ft[1] as f64) * 0.002) - BIAS[1]))
                                  + (TF[5][2] * (((ft[2] as f64) * 0.002) - BIAS[2]))
                                  + (TF[5][3] * (((ft[3] as f64) * 0.002) - BIAS[3]))
                                  + (TF[5][4] * (((ft[4] as f64) * 0.002) - BIAS[4]))
                                  + (TF[5][5] * (((ft[5] as f64) * 0.002) - BIAS[5]));
                        }

                        // write out plot to file
                        let fname = format!("/tmp/teensy{}.png", idx);
                        fig.clear_axes();
                        fig.set_terminal("png", &fname);
                        fig.axes2d()
                           .lines(&t as &[_], &fx as &[_], &[PlotOption::Color("red"),    PlotOption::Caption("FX")])
                           .lines(&t as &[_], &fy as &[_], &[PlotOption::Color("green"),  PlotOption::Caption("FY")])
                           .lines(&t as &[_], &fz as &[_], &[PlotOption::Color("blue"),   PlotOption::Caption("FZ")])
                           .lines(&t as &[_], &tx as &[_], &[PlotOption::Color("orange"), PlotOption::Caption("TX")])
                           .lines(&t as &[_], &ty as &[_], &[PlotOption::Color("pink"),   PlotOption::Caption("TY")])
                           .lines(&t as &[_], &tz as &[_], &[PlotOption::Color("plum"),   PlotOption::Caption("TZ")])
                           .set_x_label("Time (s)", &[])
                           .set_y_label("Force (N) and Torque (Nm)", &[])
                           .set_legend(Coordinate::Graph(0.9), Coordinate::Graph(0.8), &[], &[])
                           ;
                        fig.show();

                        thread::sleep(Duration::from_millis(100)); // HACK

                        // read plot back in from file
                        let mut file = File::open(&fname).unwrap();
                        data.clear();
                        file.read_to_end(&mut data).unwrap();

                        // send to browser
                        sender.send(CmdFrom::Data(format!("send kick teensy {} data:image/png;base64,{}", idx, data.to_base64(base64::STANDARD)))).unwrap();
                        idx += 1;
                    })
                }
            }

            fn step(&mut self, cmd: Option<String>) {
                self.i += 1;

                /*
                let mut b = [0u8; 4096];
                self.port.read_exact(&mut b).err().map(|e| println!("Teensy read error {:?}", e));
                */
                let mut size_buf = [0u8; 4];
                let packet_size = match self.port.read_exact(&mut size_buf) {
                    Ok(()) => {
                        if &size_buf[..3] == b"aaa" {
                            size_buf[3] as usize
                        } else {
                            errorln!("The Teensy did not send the expected packet size prefix! instead {:?}", size_buf);
                            let mut scanning = [0u8; 1];
                            let mut count = 0;
                            while count < 3 {
                                self.port.read_exact(&mut scanning).unwrap();
                                if scanning[0] == 'a' as u8 {
                                    count += 1;
                                } else {
                                    count = 0;
                                }
                            }
                            self.port.read_exact(&mut scanning).unwrap();
                            scanning[0] as usize
                        }
                    },
                    Err(e) => {
                        errorln!("Error reading packet size from the Teensy: {:?}", e);
                        return;
                    }
                };
                let mut buf = vec![0u8; packet_size];
                match self.port.read_exact(&mut buf[..]) {
                    Ok(()) => {
                        let packet = match unsafe { Packet::new(&buf) } {
                            Ok(p) => p,
                            Err(s) => {
                                errorln!("{}", s);
                                return;
                            },
                        };

                        match cmd.as_ref().map(|s| s as &str) {
                            Some("kick") => {
                                self.buf = Some(Vec::with_capacity(3000));
                            }
                            _ => {}
                        }

                        let buf_ready = if let Some(ref mut buf) = self.buf {
                            if buf.len() == 3000 {
                                true
                            } else {
                                buf.push(packet.clone());
                                false
                            }
                        } else {
                            false
                        };
                        if buf_ready {
                            self.png.send((self.tx.clone(), self.buf.take().unwrap())).unwrap();
                        }

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
