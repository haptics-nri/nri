//! Service to capture frames from the Structure Sensor

group_attr!{
    #[cfg(target_os = "linux")]

    extern crate time;
    extern crate image;
    extern crate rustc_serialize as serialize;
    use std::mem;
    use std::fs::File;
    use std::io::Write;
    use std::sync::Mutex;
    use self::image::{imageops, ImageBuffer, ColorType, FilterType};
    use self::image::png::PNGEncoder;
    use self::serialize::base64;
    use self::serialize::base64::ToBase64;
    use std::sync::mpsc::Sender;
    use ::comms::{Controllable, CmdFrom, Block, RestartableThread};

    type PngStuff = (usize, Vec<u8>, bool, (i32, i32), ColorType);

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

        /// PNG writer/sender
        png: RestartableThread<PngStuff>,
    }

    guilty!{
        impl Controllable for Structure {
            const NAME: &'static str = "structure",
            const BLOCK: Block = Block::Immediate,

            fn setup(tx: Sender<CmdFrom>, _: Option<String>) -> Structure {
                wrapper::initialize().unwrap();
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
                depth.start().unwrap();
                //ir.start().unwrap();

                let mtx = Mutex::new(tx);
                Structure {
                    device: device,
                    depth: depth,
                    ir: ir,
                    start: time::now(),
                    i: 0,

                    png: RestartableThread::new("Structure PNG thread", move |(i, unencoded, do_resize, (h, w), bd)| {
                        let mut encoded = Vec::with_capacity((w*h) as usize);
                        let to_resize = prof!("imagebuffer", ImageBuffer::<image::Rgb<u8>, _>::from_raw(w as u32, h as u32, unencoded).unwrap());
                        let (resized, ww, hh) = if do_resize {
                            let (ww, hh) = ((w as u32)/4, (h as u32)/4);
                            (prof!("resize", imageops::resize(&to_resize, ww, hh, FilterType::Nearest)), ww, hh)
                        } else {
                            (to_resize, w as u32, h as u32)
                        };
                        prof!("encode", PNGEncoder::new(&mut encoded).encode(&resized, ww, hh, bd).unwrap());
                        prof!("send", mtx.lock().unwrap().send(CmdFrom::Data(format!("send kick structure {} data:image/png;base64,{}", i, encoded.to_base64(base64::STANDARD)))).unwrap());
                    }),
                }
            }

            fn step(&mut self, cmd: Option<String>) {
                self.i += 1;

                if self.depth.is_running() {
                    prof!("depth", {
                        let frame = prof!("readFrame", self.depth.read_frame().unwrap());
                        let data: &[u8] = prof!(frame.data());

                        let mut f = File::create(format!("data/structure{}.dat", self.i)).unwrap();
                        prof!("write", f.write_all(data).unwrap());
                        match cmd.as_ref().map(|s| s as &str) {
                            Some("kick") => {
                                prof!("send to thread", self.png.send((self.i, data.into(), false, (frame.height, frame.width), ColorType::Gray(16))).unwrap());
                            },
                            Some(_) | None => ()
                        }
                    });
                }

                if self.ir.is_running() {
                    prof!("ir", {
                        let frame = prof!("readFrame", self.ir.read_frame().unwrap());
                        let data: &[u8] = prof!(frame.data());

                        let mut f = File::create(format!("data/structure_ir{}.png", self.i)).unwrap();
                        prof!("write", f.write_all(data).unwrap());
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
                if self.ir.is_running() { self.ir.stop(); }
                self.ir.destroy();
                if self.depth.is_running() { self.depth.stop(); }
                self.depth.destroy();
                self.device.close();
                wrapper::shutdown();
                let millis = (end - self.start).num_milliseconds() as f64;
                println!("{} structure frames grabbed in {} s ({} FPS)!", self.i, millis/1000.0, 1000.0*(self.i as f64)/millis);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
stub!(Structure);

