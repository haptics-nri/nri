//! Service to capture frames from the Structure Sensor

#[macro_use] extern crate utils;
#[cfg_attr(not(feature="hardware"), macro_use)] extern crate comms;

#[macro_use] extern crate guilt_by_association;
#[macro_use] extern crate macro_attr;
#[macro_use] extern crate conv;

group_attr!{
    #[cfg(feature = "hardware")]

    extern crate scribe;
    extern crate time;
    extern crate libc;
    extern crate image;
    extern crate rustc_serialize as serialize;
    use std::{mem, slice};
    use std::process::Command;
    use std::sync::{Arc, Mutex, Condvar};
    use std::sync::mpsc::Sender;
    use time::Duration;
    use image::{imageops, ImageBuffer, ColorType, FilterType};
    use image::png::PNGEncoder;
    use serialize::base64;
    use serialize::base64::ToBase64;
    use comms::{Controllable, CmdFrom, Block, RestartableThread};
    use scribe::Writer;
    use utils::*;

    type PngData = (usize, Vec<u8>, bool, (i32, i32), ColorType);
    type WatchdogData = (Arc<(Mutex<bool>, Condvar)>, String, Duration);

    mod wrapper;

    /// Controllable struct for the camera
    pub struct Structure {
        /// Private handle to the device
        device: wrapper::Device,

        /// Private handle to the depth data stream
        depth: wrapper::VideoStream,

        /// Private handle to the raw IR data stream
        ir: wrapper::VideoStream,

        /// Time that setup() was last called (used for calculating frame rates)
        start: time::Tm,

        /// Number of frames captured since setup() was last called (used for calculating frame rates)
        i: usize,

        /// Whether we are currently recording frames to images
        writing: bool,

        /// PNG writer/sender
        png: RestartableThread<PngData>,

        /// Watchdog thread to raise the alarm when the sensor hangs
        watchdog: RestartableThread<WatchdogData>,

        /// Timestamp file handle
        stampfile: Writer<[u8]>,

        /// PNG writer handle
        writer: Writer<[u8]>,

        /// Sender to communicate with core
        tx: Sender<CmdFrom>,
    }

    impl Structure {
        fn timeout<R, F: FnOnce() -> R, S: Into<String>>(&self, dur: Duration, gerund: S, action: F) -> R {
            let pair = Arc::new((Mutex::new(false), Condvar::new()));
            let &(ref lock, ref cvar) = &*pair;
            self.watchdog.send((pair.clone(), gerund.into(), dur)).unwrap();

            let ret = action();

            *lock.lock().unwrap() = true;
            cvar.notify_one();

            ret
        }
    }

    guilty!{
        impl Controllable for Structure {
            const NAME: &'static str = "structure";
            const BLOCK: Block = Block::Immediate;

            fn setup(tx: Sender<CmdFrom>, data: Option<String>) -> Structure {
                if data.map_or(false, |s| s == "power") {
                    // The Structure Sensor behaves badly if a program terminates without calling the shutdown
                    // function. Software reset (via ioctl) does not help -- the only way is to cycle power by
                    // unplugging the device. We take advantage of the fact that it is plugged in through a USB
                    // hub, and use uhubctl (https://github.com/mvp/uhubctl) to turn it off and on again.
                    assert!(Command::new("sudo")
                                    .args(&["/home/nri/software/uhubctl/uhubctl",
                                            "-a", "cycle", // cycle power
                                            "-r", "10", // try 10 times to turn off power
                                            "-d", "1", // keep power off for 1 sec
                                            "-l", "2-3", "-p", "3"]) // USB hub at address 2-3, port 3
                                    .status().unwrap()
                                    .success());
                    Duration::milliseconds(1000).sleep();
                }

                in_original_dir(|| wrapper::initialize().unwrap()).unwrap();
                let device = wrapper::Device::new(None).unwrap();

                let depth = wrapper::VideoStream::new(&device, wrapper::OniSensorType::Depth).unwrap();
                let ir = wrapper::VideoStream::new(&device, wrapper::OniSensorType::IR).unwrap();
                println!("device = {:?}", device);
                println!("depth = {:?}", depth);
                println!("ir = {:?}", ir);
                println!("{:?}", *depth.info().unwrap());
                println!("{:?}", depth.get::<wrapper::prop::VideoMode>());
                for mode in depth.info().unwrap().video_modes() { println!("{:?}", mode); }
                depth.set::<wrapper::prop::VideoMode>(
                        wrapper::OniVideoMode {
                            pixel_format: wrapper::OniPixelFormat::Depth100um,
                            resolution_x: 640,
                            resolution_y: 480,
                            fps: 30
                        }).unwrap();
                ir.set::<wrapper::prop::VideoMode>(
                        wrapper::OniVideoMode {
                            pixel_format: wrapper::OniPixelFormat::RGB888,
                            resolution_x: 1280,
                            resolution_y: 1024,
                            fps: 30
                        }).unwrap();

                let png_tx = tx.clone();
                let wd_tx = tx.clone();
                let this = Structure {
                    device: device,
                    depth: depth,
                    ir: ir,
                    start: time::now(),
                    i: 0,
                    writing: false,
                    tx: tx,

                    png: RestartableThread::new("Structure PNG thread", move |(i, unencoded, do_resize, (h, w), bd): PngData| {
                        let mut encoded = Vec::with_capacity((w*h) as usize);

                        if do_resize {
                            let to_resize = prof!("imagebuffer", ImageBuffer::<image::Rgb<u8>, _>::from_raw(w as u32, h as u32, unencoded).unwrap());
                            let (ww, hh) = ((w as u32)/4, (h as u32)/4);
                            let resized = prof!("resize", imageops::resize(&to_resize, ww, hh, FilterType::Nearest));
                            prof!("encode", PNGEncoder::new(&mut encoded).encode(&resized, ww, hh, bd).unwrap());
                        } else {
                            let (ww, hh) = (w as u32, h as u32);
                            prof!("encode", PNGEncoder::new(&mut encoded).encode(&unencoded as &[u8], ww, hh, bd).unwrap());
                        }

                        prof!("send", png_tx.send(CmdFrom::Data(format!("send kick structure {} data:image/png;base64,{}", i, encoded.to_base64(base64::STANDARD)))).unwrap());
                    }),

                    watchdog: RestartableThread::new("Structure watchdog thread", move |(pair, gerund, timeout): WatchdogData| {
                        let &(ref lock, ref cvar) = &*pair;
                        let guard = lock.lock().unwrap();
                        if !*guard {
                            if cvar.wait_timeout(guard, timeout.to_std().unwrap()).unwrap().1.timed_out() {
                                println!("ERROR!!! Structure Sensor timed out while {}", gerund);
                                wd_tx.send(CmdFrom::Data("send msg Structure Sensor froze!".into())).unwrap();
                            }
                        }
                    }),

                    stampfile: Writer::with_file("structure_times.csv"),
                    writer: Writer::with_files("structure{}.dat"),
                };

                this.timeout(Duration::milliseconds(500), "starting depth", || this.depth.start().unwrap());
                //this.timeout(Duration::milliseconds(500), "starting IR", || this.ir.start().unwrap());

                println!("structure started!");

                this
            }

            fn step(&mut self, cmd: Option<String>) {
                self.i += 1;

                match cmd.as_ref().map(|s| s as &str) {
                    Some("disk start") => {
                        println!("Started Structure recording.");
                        self.stampfile = Writer::with_file("structure_times.csv");
                        self.writing = true;
                        self.writer.set_index(self.i);
                    },
                    Some("disk stop") => {
                        println!("Stopped Structure recording.");
                        self.writing = false;
                    },
                    _ => {},
                }

                if self.depth.is_running() {
                    prof!("depth", {
                        let frame = match prof!("readFrame", self.timeout(Duration::milliseconds(100), "getting depth frame", || self.depth.read_frame(Duration::milliseconds(100)))) {
                            Ok(frame) => frame,
                            Err(ref e) if e.code() == wrapper::OniErrorCode::TimeOut => {
                                self.tx.send(CmdFrom::Data("send msg Structure Sensor froze and will be stopped!".into())).unwrap();
                                self.timeout(Duration::seconds(2), "stopping depth", || self.depth.stop());
                                return;
                            },
                            e => e.unwrap()
                        };
                        let narrow_data: &[u8] = prof!(frame.data());
                        let data: Vec<u8> = prof!("endianness", {
                            unsafe { // flip bytes
                                let wide_data: &[u16] = slice::from_raw_parts(narrow_data as *const _ as *const u16, narrow_data.len()/2);
                                let mut wide_data_flipped: Vec<u16> = wide_data.into_iter().map(|&word| u16::from_be(word)).collect();
                                let (ptr, len, cap): (*mut u16, usize, usize) = (wide_data_flipped.as_mut_ptr(),
                                                                                 wide_data_flipped.len()       ,
                                                                                 wide_data_flipped.capacity()  );
                                mem::forget(wide_data_flipped);
                                Vec::<u8>::from_raw_parts(ptr as *mut u8, len*2, cap*2)
                            }
                        });

                        if self.writing {
                            let stamp = time::get_time();
                            self.writer.write(&data);
                            self.stampfile.write(format!("{},structure{}.dat,{:.9}\n", self.i, self.i, stamp.sec as f64 + stamp.nsec as f64 / 1_000_000_000f64).as_bytes());
                        }
                        match cmd.as_ref().map(|s| s as &str) {
                            Some("kick") => {
                                prof!("send to thread", self.png.send((self.i, data, false, (frame.height, frame.width), ColorType::Gray(16))).unwrap());
                            },
                            Some(_) | None => ()
                        }
                    });
                }

                if self.ir.is_running() {
                    prof!("ir", {
                        let frame = prof!("readFrame", self.timeout(Duration::milliseconds(100), "getting IR frame", || self.ir.read_frame(Duration::milliseconds(100)).unwrap()));
                        let data: &[u8] = prof!(frame.data());

                        if self.writing {
                            let stamp = time::get_time();
                            self.writer.write(data);
                            self.stampfile.write(format!("{},structure{}.dat,{:.9}\n", self.i, self.i, stamp.sec as f64 + stamp.nsec as f64 / 1_000_000_000f64).as_bytes());
                        }
                        match cmd.as_ref().map(|s| s as &str) {
                            Some("kick") => {
                                prof!("send to thread", self.png.send((self.i, data.into(), true, (frame.height, frame.width), ColorType::RGB(8))).unwrap());
                            },
                            Some(_) | None => ()
                        }
                    });
                }
            }

            fn teardown(&mut self) {
                let end = time::now();
                if self.ir.is_running() { self.timeout(Duration::seconds(2), "stopping IR", || self.ir.stop()); }
                self.timeout(Duration::seconds(1), "destroying IR", || self.ir.destroy());
                if self.depth.is_running() { self.timeout(Duration::seconds(2), "stopping depth", || self.depth.stop()); }
                self.timeout(Duration::seconds(2), "destroying depth", || self.depth.destroy());
                self.timeout(Duration::seconds(2), "closing device", || self.device.close());
                self.timeout(Duration::seconds(2), "shutting down", || wrapper::shutdown());
                let millis = (end - self.start).num_milliseconds() as f64;
                println!("{} structure frames grabbed in {} s ({} FPS)!", self.i, millis/1000.0, 1000.0*(self.i as f64)/millis);
            }
        }
    }
}

#[cfg(not(feature = "hardware"))]
stub!(Structure);
