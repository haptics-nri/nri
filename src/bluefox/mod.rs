//! Service to capture frames from the mvBlueFOX3 camera

#[cfg(target_os = "linux")]
mod wrapper;

extern crate time;
extern crate image;
use std::thread;
use std::mem;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use self::image::{imageops, ImageBuffer, ColorType, FilterType};
use self::image::png::PNGEncoder;
use std::sync::mpsc::{channel, Sender, SendError};
use super::comms::{Controllable, CmdFrom, RestartableThread};

type PngStuff = (usize, (usize, usize), ColorType);

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
    fn setup(tx: Sender<CmdFrom>) -> Bluefox {
        let device = wrapper::Device::new().unwrap();
        //device.request_reset();
        
        Bluefox {
            device: device,
            i: 0,
            start: time::now(),

            png: RestartableThread::new(|(i, (h, w), bd)| {
                let mut read = File::open(format!("bluefox{}.dat", i)).unwrap();
                let mut write = File::create(format!("bluefox{}.png", i)).unwrap();
                let mut unencoded = Vec::with_capacity(w*h);
                read.read_to_end(&mut unencoded);
                let mut to_resize = ImageBuffer::<image::Rgb<u8>, _>::from_raw(w as u32, h as u32, unencoded).unwrap();
                let resized = imageops::resize(&to_resize, (w as u32)/10, (h as u32)/10, FilterType::Gaussian);
                PNGEncoder::new(&mut write).encode(&resized, (w as u32)/10, (h as u32)/10, bd);
                fs::remove_file("src/web/bootstrap/img/bluefox_latest.png").unwrap_or(());
                fs::soft_link(format!("../../../../bluefox{}.png", i), "src/web/bootstrap/img/bluefox_latest.png").unwrap();
            })
        }
    }

    fn step(&mut self) -> bool {
        self.i += 1;

        let image = self.device.request().unwrap();

        let mut f = File::create(format!("bluefox{}.dat", self.i)).unwrap();
        f.write_all(image.data());
        if self.i % 50 == 0 { self.png.send((self.i, image.size(), ColorType::RGB(8))); }
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

