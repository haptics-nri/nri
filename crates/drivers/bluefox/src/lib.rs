//! Service to capture frames from the mvBlueFOX3 camera

#![recursion_limit = "1000"]

#[macro_use] extern crate guilt_by_association;
#[macro_use] extern crate utils;
#[macro_use] extern crate comms;
#[macro_use] extern crate macro_attr;
#[macro_use] extern crate conv;

group_attr!{
    #[cfg(target_os = "linux")]

    extern crate libc;
    extern crate time;
    extern crate image;
    extern crate rustc_serialize as serialize;

    extern crate scribe;

    use image::{imageops, ImageBuffer, ColorType, FilterType};
    use image::png::PNGEncoder;
    use serialize::base64;
    use serialize::base64::ToBase64;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, ATOMIC_BOOL_INIT, Ordering};
    use std::sync::mpsc::Sender;
    use comms::{Controllable, CmdFrom, Block, RestartableThread};
    use scribe::Writer;

    type PngStuff = (usize, Vec<u8>, (usize, usize), ColorType);

    mod wrapper;
    use self::wrapper::settings::*;

    static SETTINGS_DONE: AtomicBool = ATOMIC_BOOL_INIT;

    /// Controllable struct for the camera
    pub struct Bluefox {
        /// Private device handle
        device: wrapper::Device,

        /// Time that setup() was last called (used for calculating frame rates)
        start: time::Tm,

        /// Number of frames captured since setup() was last called (used for calculating frame rates)
        i: usize,
        writing: bool,

        /// PNG writer rebootable thread
        png: RestartableThread<PngStuff>,

        /// Timestamp file handle
        stampfile: Writer<[u8]>,

        writer: Writer<[u8]>
    }

    guilty!{
        impl Controllable for Bluefox {
            const NAME: &'static str = "bluefox",
            const BLOCK: Block = Block::Immediate,

            fn setup(tx: Sender<CmdFrom>, data: Option<String>) -> Bluefox {
                let device = wrapper::Device::new().unwrap();
                device.request_reset().unwrap();

                let mut fps = 7.5;
                let mut format = (CameraPixelFormat::RGB8, DestPixelFormat::Auto);
                if let Some(ref data) = data {
                    let mut parts = data.split(",");

                    if let Some(fps_str) = parts.next() {
                        if let Ok(fps_num) = fps_str.parse::<f64>() {
                            fps = fps_num;
                        } else {
                            println!("WARNING: invalid FPS {:?}", fps_str);
                        }

                        if let Some(format_str) = parts.next() {
                            match format_str {
                                "raw" => format = (CameraPixelFormat::BayerGR16, DestPixelFormat::Raw),
                                "rgb" => {},
                                _     => println!("WARNING: invalid pixel format {:?}", format_str)
                            }
                        }
                    }
                }

                if SETTINGS_DONE.load(Ordering::SeqCst) {
                    // HACK: driver always returns error if we try to modify settings again
                    println!("settings already set, not modifying");
                    if data.is_some() {
                        println!("WARNING: ignoring passed-in settings");
                    }
                } else {
                    println!("BEFORE:\noffset = ({}, {})\nheight = {}\nwidth = {}\npixel format = {:?} -> {:?}\nframe rate = ({}, {})",
                             device.get_offset_x().unwrap(),
                             device.get_offset_y().unwrap(),
                             device.get_height().unwrap(),
                             device.get_width().unwrap(),
                             device.get_cam_format().unwrap(), device.get_dest_format().unwrap(),
                             device.get_acq_fr_enable().unwrap(), device.get_acq_fr().unwrap());
                    device.set_offset_x(0).unwrap();
                    device.set_offset_y(0).unwrap();
                    device.set_height(1200).unwrap();
                    device.set_width(1600).unwrap();
                    device.set_acq_fr_enable(true).unwrap();
                    device.set_acq_fr(fps).unwrap();
                    device.set_cam_format(format.0).unwrap();
                    device.set_dest_format(format.1).unwrap();
                    println!("AFTER:\noffset = ({}, {})\nheight = {}\nwidth = {}\npixel format = {:?} -> {:?}\nframe rate = ({}, {})",
                             device.get_offset_x().unwrap(),
                             device.get_offset_y().unwrap(),
                             device.get_height().unwrap(),
                             device.get_width().unwrap(),
                             device.get_cam_format().unwrap(), device.get_dest_format().unwrap(),
                             device.get_acq_fr_enable().unwrap(), device.get_acq_fr().unwrap());
                    SETTINGS_DONE.store(true, Ordering::SeqCst);
                }

                let mtx = Mutex::new(tx);
                Bluefox {
                    device: device,
                    i: 0,
                    writing: false,
                    start: time::now(),

                    png: RestartableThread::new("Bluefox PNG thread",
                                                move |(i, unencoded, (h, w), bd)| {
                        let mut encoded = Vec::with_capacity(w*h);
                        let to_resize = prof!("imagebuffer",
                                              ImageBuffer::<image::Rgb<u8>, _>::from_raw(w as u32,
                                                                                         h as u32,
                                                                                         unencoded)
                                              .unwrap());
                        let (ww, hh) = ((w as u32)/4, (h as u32)/4);
                        let resized = prof!("resize",
                                            imageops::resize(&to_resize,
                                                             ww,
                                                             hh,
                                                             FilterType::Nearest));
                        prof!("encode",
                              PNGEncoder::new(&mut encoded).encode(&resized, ww, hh, bd).unwrap());
                        prof!("send",
                              mtx
                                .lock()
                                .unwrap()
                                .send(
                                    CmdFrom::Data(
                                        format!("send kick bluefox {} data:image/png;base64,{}",
                                                i,
                                                prof!("base64",
                                                      encoded.to_base64(base64::STANDARD)))))
                                .unwrap());
                    }),

                    stampfile: Writer::with_file("bluefox_times.csv"),
                    writer: Writer::with_files("bluefox{}.dat"),
                }
            }

            fn step(&mut self, data: Option<String>) {
                self.i += 1;

                match data.as_ref().map(|s| s as &str) {
                    Some("disk start") => {
                        println!("Started Bluefox recording.");
                        self.writing = true;
                        self.writer.set_index(self.i);
                    },
                    Some("disk stop") => {
                        println!("Stopped Bluefox recording.");
                        self.writing = false;
                    },
                    Some(_) | None => ()
                }

                let image = self.device.request().unwrap();

                if self.writing {
                    let stamp = time::get_time();
                    self.writer.write(image.data());
                    self.stampfile.write(format!("{},bluefox{}.dat,{:.9}\n",
                                                 self.i,
                                                 self.i,
                                                 (stamp.sec as f64
                                                  + stamp.nsec as f64
                                                  / 1_000_000_000f64))
                                         .as_bytes());
                }

                match data.as_ref().map(|s| s as &str) {
                    Some("kick") => {
                        //self.device.set_reverse_x(!self.device.get_reverse_x().unwrap());
                        //self.device.set_reverse_y(!self.device.get_reverse_y().unwrap());
                        println!("buf = {:?}", image.buf);
                        prof!("send to thread",
                              self.png.send((self.i,
                                             image.data().into(),
                                             image.size(),
                                             ColorType::RGB(8)))
                              .unwrap())
                    },
                    _ => {}
                }
                /*
                PNGEncoder::new(&mut f).encode(image.data(),
                                               image.size().1 as u32,
                                               image.size().0 as u32,
                                               ColorType::RGB(8));
                */
            }

            fn teardown(&mut self) {
                self.png.join();
                let end = time::now();
                //device.request_reset();
                self.device.close().unwrap();
                let millis = (end - self.start).num_milliseconds() as f64;
                println!("{} bluefox frames grabbed in {} s ({} FPS)!",
                         self.i,
                         millis/1000.0,
                         1000.0*(self.i as f64)/millis);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
stub!(Bluefox);
