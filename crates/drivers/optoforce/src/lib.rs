//! Service to read data from the OptoForce sensor
//!
//! # Optoforce drivers
//!
//! The driver SDK shipped by Optoforce is crap. The example barely compiles, depends on Qt (for
//! _serial port access_ of all things), and core dumps the first time it is run (after that it
//! freezes instead). The precompiled GUI application actually runs, which is a major step up, but
//! it was compiled with an absolute path to the fonts directory in someone's home directory, so it
//! requires a symlink under /home or an `LD_PRELOAD` library to display any characters. With that
//! minor annoyance fixed, it shows numbers and plots that confirm the sensor works! It can even
//! log values to a file, which may be useful for sanity checks later. But this isn't really good
//! enough for our use case, unless we did some terrible hack such as starting the GUI to write to
//! a file and having the NRI supervisor tail the file.
//!
//! Luckily, third-party drivers exist. A quick search found [liboptoforce][liboptoforce], out of
//! ETH Zurich, freely available on Github. I installed the software from their PPA. (The source is
//! in a git submodule.  Unfortunately building it requires ETH Zurich's own build system, called
//! ReMake, which I also checked out into a submodule, but I can't get that to build, so I can't
//! build liboptoforce either. Therefore, PPA it is.) I was able to compile the example program and
//! it shows numbers from the sensor!
//!
//! The ETH Zurich package includes a configuration program, which can set the sensor sample speed
//! (among other things), and a statistics program which can measure it (among other things).
//! Unfortunately, when to set to 1 kHz (the maximum) the sensor sends readings at only 500 Hz.
//! This is probably still enough for us. A calibration program is also included, which will be
//! useful for zeroing the sensor.
//!
//! - Commands to install liboptoforce:
//! <code>
//! $ sudo apt-add-repository ppa:ethz-asl/drivers
//! $ sudo apt-get update
//! $ sudo apt-get install liboptoforce*
//! </code>
//! - To compile and run the sample program:
//! <code>
//! $ sudo apt-get install libboost-{system,thread,signals,chrono,program-options}-dev
//! $ cd liboptoforce/src/bin
//! $ make
//! $ ./configure -d /dev/ttyACM0 -s 1000
//! $ ./statistics -d /dev/ttyACM0 -s
//! $ ./dump_readings -d /dev/ttyACM0 -s
//! $ ./calibrate -d /dev/ttyACM0
//! </code>
//!
//! [liboptoforce]: https://github.com/ethz-asl/liboptoforce

#[macro_use] extern crate utils;
#[cfg_attr(not(feature="hardware"), macro_use)] extern crate comms;

#[macro_use] extern crate guilt_by_association;
#[macro_use] extern crate serde_derive;

group_attr!{
    #[cfg(feature = "hardware")]

    extern crate scribe;

    extern crate time;
    extern crate libc;
    extern crate rustc_serialize as serialize;
    extern crate serde_json;
    extern crate gnuplot;

    use std::thread;
    use std::io::Read;
    use std::default::Default;
    use std::sync::mpsc::Sender;
    use std::time::Duration;
    use std::fs::File;
    use std::ptr;
    use serialize::base64::{self, ToBase64};
    use gnuplot::{Figure, PlotOption, Coordinate, AxesCommon};
    use comms::{Controllable, CmdFrom, Block, RestartableThread};
    use scribe::{Writer, Writable};

    mod wrapper;

    type PngStuff = (Sender<CmdFrom>, Vec<Packet>);

    pub struct Optoforce {
        tx: Sender<CmdFrom>,
        device: wrapper::Device,
        i: usize,
        buf: Vec<Packet>,
        png: RestartableThread<PngStuff>,
        file: Writer<Packet>,
        start: time::Tm
    }

    #[repr(packed)]
    #[allow(dead_code)]
    #[derive(Copy, Clone)]
    struct Packet {
        stamp: time::Timespec,
        xyz  : wrapper::XYZ,
    }

    unsafe impl Writable for Packet {}

    const BUF_LEN: usize = 2000;
    #[derive(Serialize)] struct Data<'a> { t: &'a [i32], fx: &'a [i32], fy: &'a [i32], fz: &'a [i32] } // FIXME #41053

    guilty!{
        impl Controllable for Optoforce {
            const NAME: &'static str = "optoforce";
            const BLOCK: Block = Block::Period(1_000_000);

            fn setup(tx: Sender<CmdFrom>, _: Option<String>) -> Optoforce {
                let dev = wrapper::Device::new(Default::default());
                dev.connect(wrapper::ConnectOptions { path: "/dev/ttyOPTO", ..Default::default() }).unwrap();
                thread::sleep(Duration::from_millis(100));
                dev.set(wrapper::Settings::new()
                        .set_speed(wrapper::settings::Speed::Hz1000)
                       );
                println!("Optoforce settings: {:?}", dev.get().unwrap());

                // some stuff for the RestartableThread
                let mut idx = 0;
                let mut fig = Figure::new();
                //let mut data = Vec::with_capacity(10240);
                let start = time::get_time();

                Optoforce {
                    tx: tx,
                    device: dev,
                    i: 0,
                    file: Writer::with_file("optoforce.dat"),
                    start: time::now(),
                    buf: Vec::with_capacity(BUF_LEN),
                    png: RestartableThread::new("Optoforce PNG thread", move |(tx, vec): PngStuff| {
                        // process data
                        let len = vec.len();
                        let mut t  = vec![0; len];
                        let mut fx = vec![0; len];
                        let mut fy = vec![0; len];
                        let mut fz = vec![0; len];
                        macro_rules! r { ($f:expr) => { ($f * 1000.0) as i32 } }
                        for i in 0..len {
                            let diff = (vec[i].stamp - start).to_std().unwrap();
                            t[i] = r!(diff.as_secs() as f64 + (diff.subsec_nanos() as f64 / 1.0e9));
                            fx[i] = r!(vec[i].xyz.x.0 as f64);
                            fy[i] = r!(vec[i].xyz.y.0 as f64);
                            fz[i] = r!(32.0 - vec[i].xyz.z.0 as f64); // HACK
                        }

                        /*
                        // write out plot to file
                        let fname = format!("/tmp/optoforce{}.png", idx);
                        fig.clear_axes();
                        fig.set_terminal("png", &fname);
                        fig.axes2d()
                           .lines(&t as &[_], &fx as &[_], &[PlotOption::Color("red"),   PlotOption::Caption("X")])
                           .lines(&t as &[_], &fy as &[_], &[PlotOption::Color("green"), PlotOption::Caption("Y")])
                           .lines(&t as &[_], &fz as &[_], &[PlotOption::Color("blue"),  PlotOption::Caption("Z")])
                           .set_x_label("Time (s)", &[])
                           .set_y_label("Force (N)", &[])
                           .set_legend(Coordinate::Graph(0.9), Coordinate::Graph(0.8), &[], &[])
                           ;
                        fig.show();

                        thread::sleep(Duration::from_millis(100)); // HACK

                        // read plot back in from file
                        let mut file = File::open(&fname).unwrap();
                        data.clear();
                        file.read_to_end(&mut data).unwrap();

                        // send to browser
                        tx.send(CmdFrom::Data(format!("send kick optoforce {} data:image/png;base64,{}", idx, data.to_base64(base64::STANDARD)))).unwrap();
                        */

                        tx.send(CmdFrom::Data(format!("send kick optoforce {} {}", idx, serde_json::to_string(&Data { t: &t, fx: &fx, fy: &fy, fz: &fz }).unwrap()))).unwrap();
                        idx += 1;
                    })
                }
            }

            fn step(&mut self, cmd: Option<String>) {
                let packet = Packet {
                    stamp: time::get_time(),
                    xyz: self.device.read()
                };

                match cmd.as_ref().map(|s| s as &str) {
                    Some("kick") => {
                        println!("Opto: transmitting plot");
                        self.png.send((self.tx.clone(), self.buf.clone())).unwrap();
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
                self.i += 1;
            }

            fn teardown(&mut self) {
                let end = time::now();
                let millis = (end - self.start).num_milliseconds() as f64;
                println!("{} optoforce frames grabbed in {} s ({} FPS)!", self.i, millis/1000.0, 1000.0*(self.i as f64)/millis);
            }
        }
    }
}

#[cfg(not(feature = "hardware"))]
stub!(Optoforce);
