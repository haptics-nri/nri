//! Service to capture frames from the Structure Sensor

group_attr!{
    #[cfg(target_os = "linux")]

    extern crate time;
    extern crate image;
    extern crate rustc_serialize as serialize;
    use std::mem;
    use std::fs::File;
    use std::io::Write;
    use self::image::{imageops, ImageBuffer, ColorType, FilterType};
    use self::image::png::PNGEncoder;
    use self::serialize::base64;
    use self::serialize::base64::ToBase64;
    use std::sync::mpsc::Sender;
    use ::comms::{Controllable, CmdFrom, Block};

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

        /// For sending stuff back up to the supervisor
        tx: Sender<CmdFrom>,
    }

    guilty!{
        impl Controllable for Structure {
            const NAME: &'static str = "structure",

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

                let start = time::now();
                let i = 0;
                Structure { tx: tx, device: device, depth: depth, ir: ir, start: start, i: i}
            }

            fn step(&mut self, cmd: Option<String>) -> Block {
                self.i += 1;

                if self.depth.is_running() {
                    prof!("depth", {
                        let frame = prof!("readFrame", self.depth.read_frame().unwrap());
                        let data: &[u8] = prof!(frame.data());

                        let fname = format!("data/structure{}.csv", self.i);
                        let mut f = File::create(&fname).unwrap();
                        let mut encoded = Vec::with_capacity(data.len());
                        prof!("PNGEncoder", PNGEncoder::new(&mut encoded).encode(data, frame.width as u32, frame.height as u32, ColorType::Gray(16)).unwrap());
                        let wide_data : &[u16] = unsafe { mem::transmute(data) };
                        for w in 0..frame.width {
                            for h in 0..frame.height {
                                f.write(format!("{}, ", wide_data[(h*frame.width + w) as usize]).as_bytes()).unwrap();
                            }
                            f.write("\n".as_bytes()).unwrap();
                        }

                        prof!("tx.send", self.tx.send(CmdFrom::Data(format!("structure {} data:image/png;base64,{}", self.i, encoded.to_base64(base64::STANDARD)))).unwrap());
                    });
                }

                if self.ir.is_running() {
                    prof!("ir", {
                        let frame = prof!("readFrame", self.ir.read_frame().unwrap());
                        let data: &[u8] = prof!(frame.data());

                        let mut encoded = Vec::with_capacity(data.len());
                        let to_resize = prof!("imagebuffer", ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_raw(frame.width as u32, frame.height as u32, data.into()).unwrap());
                        let (ww, hh) = ((frame.width as u32)/4, (frame.height as u32)/4);
                        let resized = prof!("resize", imageops::resize(&to_resize, ww, hh, FilterType::Nearest));
                        prof!("encode", PNGEncoder::new(&mut encoded).encode(&resized, ww as u32, hh as u32, ColorType::RGB(8)).unwrap());

                        if cmd == Some("kick".to_string()) {
                            let fname = format!("data/structure_ir{}.png", self.i);
                            let mut f = File::create(&fname).unwrap();
                            prof!("write", PNGEncoder::new(&mut f).encode(data, frame.width as u32, frame.height as u32, ColorType::RGB(8)).unwrap());
                            println!("wrote IR frame {}", self.i);
                        }

                        prof!("send", self.tx.send(CmdFrom::Data(format!("structure {} data:image/png;base64,{}", self.i, encoded.to_base64(base64::STANDARD)))).unwrap());
                    });
                }

                Block::Immediate
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

