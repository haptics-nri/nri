//! Service to capture frames from the mvBlueFOX3 camera

#![recursion_limit = "1000"]

#[macro_use] extern crate guilt_by_association;
#[macro_use] extern crate utils;
#[cfg_attr(not(feature="hardware"), macro_use)] extern crate comms;

group_attr!{
    #[cfg(feature = "hardware")]

    extern crate time;
    extern crate image;
    extern crate rustc_serialize as serialize;
    extern crate serde_json;

    extern crate scribe;
    extern crate bluefox_sys as ll;

    use image::{imageops, ImageBuffer, ColorType, FilterType};
    use image::png::PNGEncoder;
    use serialize::base64;
    use serialize::base64::ToBase64;
    use std::fs::File;
    use std::io::{Read, Write};
    use std::process::{Command, Stdio};
    use std::sync::Mutex;
    use std::sync::mpsc::Sender;
    use comms::{Controllable, CmdFrom, Block, RestartableThread};
    use scribe::Writer;
    use ll::Device;
    use ll::settings::*;

    type PngStuff = (usize, Vec<u8>, (usize, usize), ColorType);

    /// Controllable struct for the camera
    pub struct Bluefox {
        /// Private device handle
        device: Device,

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

    fn set_settings(settings: Settings) {
        let mut child = Command::new("cargo")
                                .args(&["run",
                                        if cfg!(debug_assertions) { "--debug" } else { "--release" },
                                        "--bin",
                                        "bluefox-settings"])
                                .stdin(Stdio::piped())
                                .spawn().unwrap();

        let settings_str = serde_json::to_string(&settings).unwrap();
        writeln!(child.stdin.as_mut().unwrap(), "{}", settings_str).unwrap();
        
        assert!(child.wait().unwrap().success());
    }

    guilty!{
        impl Controllable for Bluefox {
            const NAME: &'static str = "bluefox";
            const BLOCK: Block = Block::Immediate;

            fn setup(tx: Sender<CmdFrom>, data: Option<String>) -> Bluefox {
                let mut fps = 15.0;
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

                let mut settings_file = utils::in_original_dir(|| File::open("crates/drivers/bluefox/camera_settings.json").unwrap()).unwrap();
                let mut settings_data = String::new();
                settings_file.read_to_string(&mut settings_data).unwrap();
                let settings = serde_json::from_str(&settings_data).unwrap();
                set_settings(Settings {
                    acq_fr: Some(fps),
                    cam_format: Some(format.0),
                    dest_format: Some(format.1),
                    ..settings });

                let device = Device::new().unwrap();
                device.request_reset().unwrap();

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
                        self.stampfile = Writer::with_file("bluefox_times.csv");
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

#[cfg(not(feature = "hardware"))]
stub!(Bluefox);
