//! Service to capture frames from the mvBlueFOX3 camera

#![recursion_limit = "1000"]

#[macro_use] extern crate guilt_by_association;
#[macro_use] extern crate utils;
#[cfg_attr(not(feature="hardware"), macro_use)] extern crate comms;

group_attr!{
    #[cfg(feature = "hardware")]

    #[macro_use] extern crate lazy_static;
    extern crate time;
    extern crate image;
    extern crate rustc_serialize as serialize;
    extern crate serde_json;

    extern crate scribe;
    extern crate bluefox_sys as ll;

    use image::{imageops, ImageBuffer, ColorType, FilterType, Pixel};
    use image::png::PNGEncoder;
    use serialize::base64;
    use serialize::base64::ToBase64;
    use std::{fs, thread};
    use std::sync::{Mutex, RwLock};
    use std::sync::mpsc::Sender;
    use std::time::Duration;
    use comms::{Controllable, CmdFrom, Block, RestartableThread};
    use utils::config;
    use scribe::Writer;
    use ll::Device;
    use ll::settings::*;

    type PngStuff = (usize, Vec<u8>, (usize, usize), ColorType, Option<usize>);

    /// Controllable struct for the camera
    pub struct Bluefox {
        /// Private device handle
        device: Device,

        /// Time that setup() was last called (used for calculating frame rates)
        start: time::Tm,

        /// Number of frames captured since setup() was last called (used for calculating frame rates)
        i: usize,
        writing: bool,
        balanced: Option<usize>,

        /// PNG writer rebootable thread
        png: RestartableThread<PngStuff>,

        /// Timestamp file handle
        stampfile: Writer<[u8]>,

        writer: Writer<[u8]>
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

                lazy_static! {
                    static ref SETTINGS: RwLock<Settings> = RwLock::new(Settings::default());
                }
                let settings_data = utils::in_original_dir("read bluefox settings", || utils::slurp(config::BLUEFOX_SETTINGS)).unwrap().unwrap();
                let default_settings: Settings = serde_json::from_str(&settings_data).unwrap();

                let txc = Mutex::new(tx.clone());
                utils::watch(default_settings.clone(),
                             &SETTINGS,
                             utils::in_original_dir("find bluefox dir", || fs::canonicalize(config::BLUEFOX_SETTINGS)).unwrap().unwrap().parent().unwrap(),
                             "json",
                             move |_, path| {
                                 println!("BLUEFOX: updating settings from {}", path.display());
                                 thread::sleep(Duration::from_millis(500));
                                 let data = utils::in_original_dir("read bluefox settings", || utils::slurp(path).unwrap()).unwrap();
                                 txc.lock().unwrap().send(CmdFrom::Data(format!("to bluefox settings {}", data))).unwrap();
                             });

                let settings = Settings {
                    acq_fr: Some(fps),
                    cam_format: Some(format.0),
                    dest_format: Some(format.1),
                    white_balance: Some(WhiteBalanceMode::Once),
                    ..default_settings
                };

                let mut device = Device::new().unwrap();
                device.request_reset().unwrap();
                device.set(&settings).unwrap();

                let mtx = Mutex::new(tx);
                Bluefox {
                    device: device,
                    i: 0,
                    writing: false,
                    balanced: Some(0),
                    start: time::now(),

                    png: RestartableThread::new("Bluefox PNG thread",
                                                move |(i, unencoded, (h, w), bd, id)| {
                        let mut encoded = Vec::with_capacity(w*h);
                        let to_resize = prof!("imagebuffer",
                                              ImageBuffer::<image::Rgb<u8>, _>::from_raw(w as u32,
                                                                                         h as u32,
                                                                                         unencoded)
                                              .unwrap());

                        let brightness = to_resize.pixels()
                                                  .fold(0.0, |acc, rgb| acc + rgb.to_luma()[0] as f64);
                        println!("brightness={}", brightness);

                        //let (ww, hh) = (200, 150);
                        let (ww, hh) = (w as u32, h as u32);
                        let resized = prof!("resize", 
                                            if (w as u32, h as u32) == (ww, hh) {
                                                to_resize
                                            } else {
                                                imageops::resize(&to_resize,
                                                                 ww,
                                                                 hh,
                                                                 FilterType::Nearest)
                                            });

                        prof!("encode",
                              PNGEncoder::new(&mut encoded).encode(&resized, ww, hh, bd).unwrap());
                        let id_str = if let Some(id) = id { format!(" {}", id) } else { String::new() };
                        prof!("send",
                              mtx
                                .lock()
                                .unwrap()
                                .send(
                                    CmdFrom::Data(
                                        format!("send{} kick bluefox {} data:image/png;base64,{}",
                                                id_str, i,
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
                    /*
                    Some("auto") => {
                        println!("bluefox auto brightness");
                        let mut set = self.device.get();
                        let greys = [60, 65, 70, 75, 80, 85, 90, 95, 100];
                        let mut brightness = [0.0; 9];
                        for i in 0..greys.len() {
                            let grey = greys[i];
                            println!("\ttrying {}...", grey);
                            set.average_grey = grey;
                            self.device.request_reset().unwrap();
                            self.device.set(&set).unwrap();
                            thread::sleep_ms(1000);
                            let image = self.device.request().unwrap();
                            let (h, w) = image.size();
                            let image = prof!("imagebuffer",
                                              ImageBuffer::<image::Rgb<u8>, _>::from_raw(w as u32,
                                                                                         h as u32,
                                                                                         image.data().into())
                                              .unwrap());
                            brightness[i] = image.pixels()
                                .fold(0.0, |acc, rgb| acc + rgb.to_luma()[0] as f64);
                        }
                        let grey = greys[brightness.iter().map(|b| (b - 245000000.0).abs()).enumerate().min_by_key(|&(_, d)| d).0.unwrap()];
                        println!("\tbest brightness at grey={}", grey);
                        set.average_grey = grey;
                        self.device.request_reset().unwrap();
                        self.device.set(&set).unwrap();
                    },
                    */
                    Some(s) if s.starts_with("settings") => {
                        if self.writing {
                            println!("BLUEFOX: currently writing, ignoring new settings");
                        } else if self.balanced.is_some() {
                            println!("BLUEFOX: not white balanced yet, ignoring new setings");
                        } else {
                            println!("BLUEFOX: applying new settings");
                            let set: Settings = serde_json::from_str(&s[9..]).unwrap();
                            self.device.request_reset().unwrap();
                            self.device.set(&set).unwrap();
                            self.balanced = Some(self.i);
                        }
                    },
                    Some(_) | None => ()
                }

                if let Some(from) = self.balanced {
                    if self.i - from == 30 /* 2 seconds */ {
                        // turn off auto crap
                        self.device.set(&Settings {
                            auto_gain: Some(false),
                            auto_exposure: Some(false),
                            white_balance: Some(WhiteBalanceMode::Off),
                            ..Default::default() }).unwrap();

                        let wb = self.device.get_all_wb().unwrap();
                        println!("BLUEFOX: finished white balance: {:?} (r={}, b={}, gain={}, exp={})",
                                 wb.mode, wb.red, wb.blue,
                                 self.device.get_gain().unwrap(), self.device.get_exposure_time().unwrap());

                        self.balanced = None;
                    }
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
                    Some(s) if s.starts_with("kick") => {
                        //self.device.set_reverse_x(!self.device.get_reverse_x().unwrap());
                        //self.device.set_reverse_y(!self.device.get_reverse_y().unwrap());
                        println!("buf = {:?}", image.buf);
                        prof!("send to thread",
                              self.png.send((self.i,
                                             image.data().into(),
                                             image.size(),
                                             ColorType::RGB(8),
                                             s.split(' ').skip(1).next().map(|s| s.parse().unwrap())))
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
