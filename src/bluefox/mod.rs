//! Service to capture frames from the mvBlueFOX3 camera

#[cfg(target_os = "linux")]
mod wrapper;

extern crate time;
extern crate image;
extern crate rustc_serialize as serialize;
use std::thread;
use std::mem;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use self::image::{imageops, ImageBuffer, ColorType, FilterType};
use self::image::png::PNGEncoder;
use self::serialize::base64;
use self::serialize::base64::ToBase64;
use std::sync::Mutex;
use std::sync::mpsc::{channel, Sender, SendError};
use super::comms::{Controllable, CmdFrom, RestartableThread};

type PngStuff = (Vec<u8>, (usize, usize), ColorType);

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
impl Controllable for Bluefox {
    fn setup(tx: Sender<CmdFrom>, _: Option<String>) -> Bluefox {
        let device = wrapper::Device::new().unwrap();
        //device.request_reset();
        
        let mtx = Mutex::new(tx);
        Bluefox {
            device: device,
            i: 0,
            start: time::now(),

            png: RestartableThread::new(move |(unencoded, (h, w), bd)| {
                let mut encoded = Vec::with_capacity(w*h);
                let mut to_resize = ImageBuffer::<image::Rgb<u8>, _>::from_raw(w as u32, h as u32, unencoded).unwrap();
                let resized = imageops::resize(&to_resize, (w as u32)/10, (h as u32)/10, FilterType::Gaussian);
                PNGEncoder::new(&mut encoded).encode(&resized, (w as u32)/10, (h as u32)/10, bd);
                mtx.lock().unwrap().send(CmdFrom::Data(format!("bluefox data:image/png;base64,{}", encoded.to_base64(base64::STANDARD))));
            })
        }
    }

    fn step(&mut self, _: Option<String>) -> bool {
        self.i += 1;

        let image = self.device.request().unwrap();

        //let mut f = File::create(format!("bluefox{}.dat", self.i)).unwrap();
        //f.write_all(image.data());
        if self.i % 50 == 0 { self.png.send((image.data().into(), image.size(), ColorType::RGB(8))); }
        //PNGEncoder::new(&mut f).encode(image.data(), image.size().1 as u32, image.size().0 as u32, ColorType::RGB(8));

        false
    }

    fn teardown(&mut self) {
        self.png.join();
        let end = time::now();
        //device.request_reset();
        self.device.close();
        let millis = (end - self.start).num_milliseconds() as f64;
        println!("{} bluefox frames grabbed in {} s ({} FPS)!", self.i, millis/1000.0, 1000.0*(self.i as f64)/millis);
    }
}

#[cfg(not(target_os = "linux"))]
stub!(Bluefox);

