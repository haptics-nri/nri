//! Service to capture frames from the mvBlueFOX3 camera

group_attr!{
    #[cfg(target_os = "linux")]

    extern crate time;
    extern crate image;
    extern crate rustc_serialize as serialize;
    use std::fs::File;
    use std::io::Write;
    use self::image::{imageops, ImageBuffer, ColorType, FilterType};
    use self::image::png::PNGEncoder;
    use self::serialize::base64;
    use self::serialize::base64::ToBase64;
    use std::sync::Mutex;
    use std::sync::mpsc::Sender;
    use ::comms::{Controllable, CmdFrom, RestartableThread};

    type PngStuff = (usize, Vec<u8>, (usize, usize), ColorType);

    mod wrapper;

    /// Controllable struct for the camera
    pub struct Bluefox {
        /// Private device handle
        device: wrapper::Device,

        /// Time that setup() was last called (used for calculating frame rates)
        start: time::Tm,

        /// Number of frames captured since setup() was last called (used for calculating frame rates)
        i: usize,

        /// PNG writer rebootable thread
        png: RestartableThread<PngStuff>,
    }

    guilty!{
        impl Controllable for Bluefox {
            const NAME: &'static str = "bluefox",

            fn setup(tx: Sender<CmdFrom>, _: Option<String>) -> Bluefox {
                let device = wrapper::Device::new().unwrap();
                //device.request_reset();
                
                println!("height = {}\nwidth = {}\npixel format = {:?}", device.get_height().unwrap(), device.get_width().unwrap(), device.get_pixel_format().unwrap());
                
                let mtx = Mutex::new(tx);
                Bluefox {
                    device: device,
                    i: 0,
                    start: time::now(),

                    png: RestartableThread::new("Bluefox PNG thread", move |(i, unencoded, (h, w), bd)| {
                        let mut encoded = Vec::with_capacity(w*h);
                        let to_resize = prof!("imagebuffer", ImageBuffer::<image::Rgb<u8>, _>::from_raw(w as u32, h as u32, unencoded).unwrap());
                        let (ww, hh) = ((w as u32)/4, (h as u32)/4);
                        let resized = prof!("resize", imageops::resize(&to_resize, ww, hh, FilterType::Nearest));
                        prof!("encode", PNGEncoder::new(&mut encoded).encode(&resized, ww, hh, bd).unwrap());
                        prof!("send", mtx.lock().unwrap().send(CmdFrom::Data(format!("bluefox {} data:image/png;base64,{}", i, prof!("base64", encoded.to_base64(base64::STANDARD))))).unwrap());
                    })
                }
            }

            fn step(&mut self, data: Option<String>) -> bool {
                self.i += 1;

                let image = self.device.request().unwrap();

                let mut f = File::create(format!("data/bluefox{}.dat", self.i)).unwrap();
                prof!("write", f.write_all(image.data()).unwrap());
                match data.as_ref().map(|s| s as &str) {
                    Some("kick") => {
                        //self.device.set_reverse_x(!self.device.get_reverse_x().unwrap());
                        //self.device.set_reverse_y(!self.device.get_reverse_y().unwrap());
                        println!("buf = {:?}", image.buf);
                        prof!("send to thread", self.png.send((self.i, image.data().into(), image.size(), ColorType::RGB(8))).unwrap())
                    },
                    Some(_) | None => ()
                }
                //PNGEncoder::new(&mut f).encode(image.data(), image.size().1 as u32, image.size().0 as u32, ColorType::RGB(8));

                false
            }

            fn teardown(&mut self) {
                self.png.join();
                let end = time::now();
                //device.request_reset();
                self.device.close().unwrap();
                let millis = (end - self.start).num_milliseconds() as f64;
                println!("{} bluefox frames grabbed in {} s ({} FPS)!", self.i, millis/1000.0, 1000.0*(self.i as f64)/millis);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
stub!(Bluefox);

