//! Service to read data from the Teensy and attached sensors

#[macro_use] extern crate utils;
#[macro_use] extern crate comms;

#[macro_use] extern crate guilt_by_association;
#[macro_use] extern crate unborrow;
#[macro_use] extern crate macro_attr;
#[macro_use] extern crate conv;
#[macro_use] extern crate serde_derive;

macro_attr! {
    /// Which end effector is in use (i.e. not parked)
    #[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, TryFrom!(u8))]
    pub enum ParkState {
        /// All end effectors parked
        None = 0,
        /// The Optoforce is out
        OptoForce = 1,
        /// The rigid stick is out
        Stick = 2,
        /// The Biotac is out
        BioTac = 16,
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

#[cfg(not(feature = "hardware"))]
impl ParkState {
    pub fn metermaid() -> Option<ParkState> {
        Some(ParkState::Stick)
    }
}

group_attr!{
    #[cfg(feature = "hardware")]

    extern crate scribe;

    extern crate serial;
    extern crate time;
    extern crate rustc_serialize as serialize;
    extern crate serde_json;
    extern crate gnuplot;

    use comms::{Controllable, CmdFrom, Block, RestartableThread};
    use scribe::{Writer, Writable};
    use std::io::{self, Read, Write};
    use std::fs::File;
    use std::{cmp, thread};
    use std::sync::mpsc::Sender;
    use std::{u8, ptr, mem};
    use std::fmt::{self, Display, Debug, Formatter};
    use time::Duration;
    use std::num::Wrapping;
    use std::sync::atomic::{AtomicUsize, AtomicBool, ATOMIC_USIZE_INIT, ATOMIC_BOOL_INIT, Ordering};
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
            settings.set_flow_control(serial::FlowNone);
            Ok(())
        }).unwrap();
        port.set_timeout(Duration::milliseconds(100).to_std().unwrap()).unwrap();
        if true {
            Box::new(port.coffee(File::create("teensydump.dat").unwrap()))
        } else {
            Box::new(port)
        }
    }

    static PARK_STATE: AtomicUsize = ATOMIC_USIZE_INIT;
    static RUNNING: AtomicBool = ATOMIC_BOOL_INIT;

    impl ParkState {
        pub fn metermaid() -> Option<ParkState> {
            let val = if RUNNING.load(Ordering::SeqCst) {
                PARK_STATE.load(Ordering::SeqCst) as u8
            } else {
                let mut port = serialport();
                let mut buf = [0u8; 1];

                let read = utils::retry(Some("[teensy] reading ParkState"), 3, Duration::milliseconds(50),
                    || {
                        port.write_all(&['4' as u8]).unwrap();
                        port.read_exact(&mut buf).ok().map(Some)
                    },
                    || {
                        None
                    });
                match read {
                    Some(()) => buf[0],
                    None => return None
                }
            };

            let masked = !val & !0b1110_1100;
            println!("TEENSY: converting ParkState from 0x{:X} (0x{:X})", val, masked);

            match ParkState::try_from(masked) {
                Ok(ps) => Some(ps),
                Err(_) => Some(ParkState::Multiple)
            }
        }
    }

    type PngStuff = (Sender<CmdFrom>, Vec<Packet>);

    pub struct Teensy {
        port: Box<StaticReadWrite>,
        file: Writer<Packet>,
        i: usize,
        buf: Vec<Packet>,
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
        dt     : (u16, u16),
        ft     : [u8; 31],
        n_acc  : u8,
        n_gyro : u8,
        imu    : [XYZ<i16>; 63]
    }
    impl Copy for Packet {}
    impl Clone for Packet { fn clone(&self) -> Packet { *self } }

    unsafe impl Writable for Packet {}

    impl Packet {
        unsafe fn new(buf: &[u8]) -> Result<Packet, String> {
            fn checksum(buf: &[u8]) -> Result<(), String> {
                //let sum = buf[..buf.len()-1].into_iter().fold(0, |a, b| { u8::wrapping_add(a, *b) });
                let mut sum = Wrapping(0u8);
                for b in &buf[..buf.len()-1] { sum += Wrapping(*b); }
                let sum = sum.0;
                match buf[buf.len()-1] {
                    s if s == sum => Ok(()),
                    s => Err(format!("Received Teensy packet with wrong checksum (it says {}, I calculate {})!", s, sum)),
                }
            }

            unsafe fn only_analog(buf: &[u8]) -> Packet {
                let mut p: Packet = Packet {
                    stamp  : time::get_time(),
                    dt     : (0, 0),
                    ft     : mem::zeroed::<[u8; 31]>(),
                    n_acc  : 0,
                    n_gyro : 0,
                    imu    : mem::zeroed::<[XYZ<i16>; 63]>(),
                };
                byte_copy(buf, &mut p.ft);
                p
            }

            unsafe fn only_analog_dt(buf: &[u8]) -> Packet {
                let mut p: Packet = Packet {
                    stamp  : time::get_time(),
                    dt     : (0, 0),
                    ft     : mem::zeroed::<[u8; 31]>(),
                    n_acc  : 0,
                    n_gyro : 0,
                    imu    : mem::zeroed::<[XYZ<i16>; 63]>(),
                };
                byte_copy(&buf[..4], mem::transmute::<&mut (u16, u16), &mut [u8; 4]>(&mut p.dt));
                byte_copy(&buf[4..], &mut p.ft);
                p
            }

            unsafe fn imu_and_analog(buf: &[u8], a: usize, g: usize) -> Packet {
                let s = 2 + 6*(a + g + 1);
                let mut p: Packet = Packet {
                    stamp  : time::get_time(),
                    dt     : (0, 0),
                    ft     : mem::zeroed::<[u8; 31]>(),
                    n_acc  : a as u8,
                    n_gyro : g as u8,
                    imu    : mem::zeroed::<[XYZ<i16>; 63]>()
                };
                byte_copy(&buf[s..], &mut p.ft);
                ptr::copy::<XYZ<i16>>(buf[2..s].as_ptr() as *const XYZ<i16>, p.imu.as_mut_ptr(), (a+g+1) as usize);
                p
            }

            unsafe fn imu_and_analog_dt(buf: &[u8], a: usize, g: usize) -> Packet {
                let s = 2 + 6*(a + g + 1);
                let mut p: Packet = Packet {
                    stamp  : time::get_time(),
                    dt     : (0, 0),
                    ft     : mem::zeroed::<[u8; 31]>(),
                    n_acc  : a as u8,
                    n_gyro : g as u8,
                    imu    : mem::zeroed::<[XYZ<i16>; 63]>()
                };
                byte_copy(&buf[s..s+4], mem::transmute::<&mut (u16, u16), &mut [u8; 4]>(&mut p.dt));
                byte_copy(&buf[s+4..], &mut p.ft);
                ptr::copy::<XYZ<i16>>(buf[2..s].as_ptr() as *const XYZ<i16>, p.imu.as_mut_ptr(), (a+g+1) as usize);
                p
            }

            let mut pkt = match buf.len() {
                x if x < 31 => return Err(format!("Implausibly small packet ({}) from Teensy!", x)),
                31 => only_analog(buf),
                32 => {
                    try!(checksum(buf));
                    only_analog(buf)
                },
                35 => only_analog_dt(buf),
                36 => {
                    try!(checksum(buf));
                    only_analog_dt(buf)
                },
                x => {
                    let a = buf[0] as usize;
                    let g = buf[1] as usize;
                    match x {
                        t if t == 31 + 2 + 6*(a + g + 1) => imu_and_analog(buf, a, g),
                        t if t == 35 + 2 + 6*(a + g + 1) => imu_and_analog_dt(buf, a, g),
                        t if t == 31 + 2 + 6*(a + g + 1) + 1 => {
                            try!(checksum(buf));
                            imu_and_analog(buf, a, g)
                        },
                        t if t == 35 + 2 + 6*(a + g + 1) + 1 => {
                            try!(checksum(buf));
                            imu_and_analog_dt(buf, a, g)
                        },
                        _ => return Err(format!("Impossible packet size ({} with a={}, g={}) from Teensy!", x, a, g)),
                    }
                },
            };

            PARK_STATE.store(*pkt.ft.last().unwrap() as usize, Ordering::SeqCst);
            *pkt.ft.last_mut().unwrap() &= !0b0001_0011; // FIXME make this a const somewhere

            if pkt.dt.0 > 1000 {
                println!("Delayed packet from Teensy! Packet follows:");
                println!("{:#?}", pkt);
            }

            Ok(pkt)
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
            try!(writeln!(f, "Packet {{"));
            try!(writeln!(f, "\tstamp: {:?}", self.stamp));
            try!(writeln!(f, "\tdt: {:?}", self.dt));
            try!(writeln!(f, "\tft: {:?}", self.ft));
            try!(write!(f, "\tacc: ["));
            for i in 0..self.n_acc {
                try!(write!(f, "{:?}, ", self.imu[i as usize]));
            }
            try!(writeln!(f, "]"));
            try!(write!(f, "\tgyro: ["));
            for i in 0..self.n_gyro {
                try!(write!(f, "{:?}, ", self.imu[(self.n_acc + i) as usize]));
            }
            if self.n_acc + self.n_gyro > 0 {
                try!(writeln!(f, "\tmag: {:?}", self.imu[(self.n_acc + self.n_gyro) as usize]));
            }
            try!(writeln!(f, "]"));
            try!(write!(f, "}}"));
            Ok(())
        }
    }

    const BUF_LEN: usize = 6000;

    guilty! {
        impl Controllable for Teensy {
            const NAME: &'static str = "teensy";
            const BLOCK: Block = Block::Immediate;

            fn setup(tx: Sender<CmdFrom>, cmd: Option<String>) -> Teensy {
                match cmd.as_ref().map(|s| s as &str) {
                    Some("metermaid") => {
                        tx.send(CmdFrom::Data(format!("send status {:?}", ParkState::metermaid()))).unwrap();
                    }
                    _ => {}
                }

                let mut port = serialport();
                RUNNING.store(true, Ordering::SeqCst);
                port.write_all(&['1' as u8]).unwrap();

                // some stuff for the RestartableThread
                let mut idx = 0;
                //let mut fig = Figure::new();
                //let mut data = Vec::with_capacity(10240);
                let start = time::get_time();

                Teensy {
                    port: port,
                    file: Writer::with_file("teensy.dat"),
                    i: 0,
                    start: time::now(),
                    tx: tx,
                    buf: Vec::with_capacity(BUF_LEN),
                    png: RestartableThread::new("Teensy PNG thread", move |(sender, vec): PngStuff| {
                        let decimate = 5;

                        // process data
                        let len = vec.len();
                        let mut t  = Vec::<i32>::with_capacity(len/decimate);
                        let mut fx = Vec::<i32>::with_capacity(len/decimate);
                        let mut fy = Vec::<i32>::with_capacity(len/decimate);
                        let mut fz = Vec::<i32>::with_capacity(len/decimate);
                        //let mut tx = vec![0; len];
                        //let mut ty = vec![0; len];
                        //let mut tz = vec![0; len];
                        let mut a  = Vec::<i32>::with_capacity(len/decimate);

                        fn r(f: f64) -> i32 {
                            (f * 1000.0) as i32
                        }

                        let mut delay = 0;
                        for i in utils::step(0..len, decimate) {
                            let diff = (vec[i].stamp - start).to_std().unwrap();
                            t.push(r(diff.as_secs() as f64 + (diff.subsec_nanos() as f64 / 1.0e9)));
                            let mut ft = [(((vec[i].ft[0]  as u32) << 8) + (vec[i].ft[1]  as u32)) as i32,
                                          (((vec[i].ft[2]  as u32) << 8) + (vec[i].ft[3]  as u32)) as i32,
                                          (((vec[i].ft[4]  as u32) << 8) + (vec[i].ft[5]  as u32)) as i32,
                                          (((vec[i].ft[6]  as u32) << 8) + (vec[i].ft[7]  as u32)) as i32,
                                          (((vec[i].ft[8]  as u32) << 8) + (vec[i].ft[9]  as u32)) as i32,
                                          (((vec[i].ft[10] as u32) << 8) + (vec[i].ft[11] as u32)) as i32];
                            for val in &mut ft {
                                if *val >= 2048 {
                                    *val -= 4096;
                                }
                            }
                            let mut aa = 0.0;
                            aa += (((((vec[i].ft[18] as u32) << 8) + (vec[i].ft[19] as u32)) as i32) - 2048) as f64;
                            aa += (((((vec[i].ft[22] as u32) << 8) + (vec[i].ft[23] as u32)) as i32) - 2048) as f64;
                            aa += (((((vec[i].ft[24] as u32) << 8) + (vec[i].ft[25] as u32)) as i32) - 2048) as f64;

                            if ft[0].abs() > 10_000 {
                                println!("TEENSY: repairing spike at t={}, ft={:?}", t.last().unwrap(), ft);
                                delay = 25;
                                let origin = cmp::min(0, i - delay);
                                for j in origin..i {
                                    fx[j] = fx[origin];
                                    fy[j] = fy[origin];
                                    fz[j] = fx[origin];
                                    a[j] = a[origin];
                                }
                            }
                            if delay > 0 {
                                delay -= 1;
                                unborrow!(fx.push(fx.last().unwrap().clone()));
                                unborrow!(fy.push(fy.last().unwrap().clone()));
                                unborrow!(fz.push(fz.last().unwrap().clone()));
                                unborrow!(a.push(a.last().unwrap().clone()));
                            } else {
                                a.push(r(aa / 4096.0 * 16.0 * 9.81 / 3.0));
                                // proton mini40
                                const BIAS: [f64; 6] = [-0.1884383674, 0.2850118688, -0.180718143, -0.191009933, 0.3639300747, -0.4307167708];
                                const TF: [[f64; 6]; 6] = [[0.00679, 0.01658, -0.04923, 6.20566, 0.15882, -6.19201],
                                                           [0.11638, -7.31729, -0.04322, 3.54949, -0.08024, 3.57115],
                                                           [10.35231, 0.32653, 10.61091, 0.29668, 10.33382, 0.25761],
                                                           [0.00022, -0.0414, 0.14917, 0.02435, -0.15234, 0.01567],
                                                           [-0.16837, -0.00464, 0.08561, -0.03311, 0.08763, 0.03721],
                                                           [0.00128, -0.08962, 0.00085, -0.08785, 0.00204, -0.0879]];
                                 
                                                           
                                const SCALE: f64 = 0.002;
                                /* // STB mini40
                                const BIAS: [f64; 6] = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
                                const TF: [[f64; 6]; 6] = [[ 0.165175269,   6.193716635,    -0.05972626,    0.020033203,    -0.136667224,   -6.42215241 ],
                                      [ 0.002429674,  -3.63579423,    0.466390998,    7.308900211,    -0.18369186,    -3.65179797 ],
                                      [ -10.5385017,  0.802731009,    -10.1357248,    0.359714766,    -10.0934065,    0.442593679 ],
                                      [ 0.144765089,  -0.032574325,   0.004132077,    0.038285567,    -0.145061852,   -0.010347366],
                                      [ -0.089833077, -0.024635731,   0.165602185,    -0.009131771,   -0.080132747,   0.039589968 ],
                                      [ 0.001846317,  0.085776855,    0.005262967,    0.088317691,    0.001450272,    0.087714269 ]];
                                      */
                                

                                /* // zeroed out
                                const BIAS: [f64; 6] = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
                                const TF: [[f64; 6]; 6] = [[1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                                                           [0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
                                                           [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
                                                           [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                                                           [0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
                                                           [0.0, 0.0, 0.0, 0.0, 0.0, 1.0]];
                                */

                                fx.push(r((TF[0][0] * (((ft[0] as f64) * SCALE) - BIAS[0]))
                                        + (TF[0][1] * (((ft[1] as f64) * SCALE) - BIAS[1]))
                                        + (TF[0][2] * (((ft[2] as f64) * SCALE) - BIAS[2]))
                                        + (TF[0][3] * (((ft[3] as f64) * SCALE) - BIAS[3]))
                                        + (TF[0][4] * (((ft[4] as f64) * SCALE) - BIAS[4]))
                                        + (TF[0][5] * (((ft[5] as f64) * SCALE) - BIAS[5]))));
                                fy.push(r((TF[1][0] * (((ft[0] as f64) * SCALE) - BIAS[0]))
                                        + (TF[1][1] * (((ft[1] as f64) * SCALE) - BIAS[1]))
                                        + (TF[1][2] * (((ft[2] as f64) * SCALE) - BIAS[2]))
                                        + (TF[1][3] * (((ft[3] as f64) * SCALE) - BIAS[3]))
                                        + (TF[1][4] * (((ft[4] as f64) * SCALE) - BIAS[4]))
                                        + (TF[1][5] * (((ft[5] as f64) * SCALE) - BIAS[5]))));
                                fz.push(r((TF[2][0] * (((ft[0] as f64) * SCALE) - BIAS[0]))
                                        + (TF[2][1] * (((ft[1] as f64) * SCALE) - BIAS[1]))
                                        + (TF[2][2] * (((ft[2] as f64) * SCALE) - BIAS[2]))
                                        + (TF[2][3] * (((ft[3] as f64) * SCALE) - BIAS[3]))
                                        + (TF[2][4] * (((ft[4] as f64) * SCALE) - BIAS[4]))
                                        + (TF[2][5] * (((ft[5] as f64) * SCALE) - BIAS[5]))));
                                /*
                                tx[i] = r!((TF[3][0] * (((ft[0] as f64) * SCALE) - BIAS[0]))
                                         + (TF[3][1] * (((ft[1] as f64) * SCALE) - BIAS[1]))
                                         + (TF[3][2] * (((ft[2] as f64) * SCALE) - BIAS[2]))
                                         + (TF[3][3] * (((ft[3] as f64) * SCALE) - BIAS[3]))
                                         + (TF[3][4] * (((ft[4] as f64) * SCALE) - BIAS[4]))
                                         + (TF[3][5] * (((ft[5] as f64) * SCALE) - BIAS[5])));
                                ty[i] = r!((TF[4][0] * (((ft[0] as f64) * SCALE) - BIAS[0]))
                                         + (TF[4][1] * (((ft[1] as f64) * SCALE) - BIAS[1]))
                                         + (TF[4][2] * (((ft[2] as f64) * SCALE) - BIAS[2]))
                                         + (TF[4][3] * (((ft[3] as f64) * SCALE) - BIAS[3]))
                                         + (TF[4][4] * (((ft[4] as f64) * SCALE) - BIAS[4]))
                                         + (TF[4][5] * (((ft[5] as f64) * SCALE) - BIAS[5])));
                                tz[i] = r!((TF[5][0] * (((ft[0] as f64) * SCALE) - BIAS[0]))
                                         + (TF[5][1] * (((ft[1] as f64) * SCALE) - BIAS[1]))
                                         + (TF[5][2] * (((ft[2] as f64) * SCALE) - BIAS[2]))
                                         + (TF[5][3] * (((ft[3] as f64) * SCALE) - BIAS[3]))
                                         + (TF[5][4] * (((ft[4] as f64) * SCALE) - BIAS[4]))
                                         + (TF[5][5] * (((ft[5] as f64) * SCALE) - BIAS[5])));
                                         */
                            }
                        }

                        /*
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
                           .lines(&t as &[_], &a  as &[_], &[PlotOption::Color("brown"),  PlotOption::Caption("A" )])
                           .set_x_label("Time (s)", &[])
                           .set_y_label("Force (N) and Torque (Nm) and Acc (m/s^2)", &[])
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
                        */

                        #[derive(Serialize)] struct Data<'a> { t: &'a [i32], fx: &'a [i32], fy: &'a [i32], fz: &'a [i32], a: &'a [i32] }
                        sender.send(CmdFrom::Data(format!("send kick teensy {} {}", idx, serde_json::to_string(&Data { t: &t, fx: &fx, fy: &fy, fz: &fz, a: &a }).unwrap()))).unwrap();
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
                let mut size_buf = [0u8; 5];
                let packet_size = match self.port.read_exact(&mut size_buf) {
                    Ok(()) => {
                        if &size_buf[..3] == b"aaa" {
                            let hi = size_buf[3] as usize;
                            let lo = size_buf[4] as usize;
                            (hi << 8) + lo
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
                            let hi = scanning[0] as usize;
                            self.port.read_exact(&mut scanning).unwrap();
                            let lo = scanning[0] as usize;
                            (hi << 8) + lo
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
                                println!("Teensy: transmitting plot");
                                self.png.send((self.tx.clone(), self.buf.clone())).unwrap();
                            }
                            Some("metermaid") => {
                                self.tx.send(CmdFrom::Data(format!("send status {:?}", ParkState::metermaid()))).unwrap();
                            }
                            Some("ref int") => {
                                println!("Switching accelerometers to internal reference.");
                                self.port.write_all(&['5' as u8]).unwrap();
                            }
                            Some("ref ext") => {
                                println!("Switching accelerometers to external reference.");
                                self.port.write_all(&['6' as u8]).unwrap();
                            }
                            _ => {}
                        }

                        if self.buf.len() == self.buf.capacity() {
                            let len = self.buf.len()-1;
                            unsafe {
                                ptr::copy(&self.buf[1], &mut self.buf[0], len);
                            }
                            self.buf.truncate(len);
                        }
                        self.buf.push(packet.clone());
                        self.file.write(packet);
                    },
                    Err(e)  => errorln!("Error reading packet from Teensy: {:?}", e)
                }
            }

            fn teardown(&mut self) {
                self.port.write_all(&['2' as u8]).unwrap();
                RUNNING.store(false, Ordering::SeqCst);
                let end = time::now();
                let millis = (end - self.start).num_milliseconds() as f64;
                println!("{} Teensy packets grabbed in {} s ({} FPS)!", self.i, millis/1000.0, 1000.0*(self.i as f64)/millis);
            }
        }
    }
}

#[cfg(not(feature = "hardware"))]
stub!(Teensy);
