//! Service to capture frames from the Structure Sensor

group_attr!{
    #[cfg(target_os = "linux")]

    extern crate time;
    extern crate image;
    extern crate rustc_serialize as serialize;
    use std::fs;
    use std::fs::File;
    use std::io::Write;
    use self::image::ColorType;
    use self::image::png::PNGEncoder;
    use self::serialize::base64;
    use self::serialize::base64::ToBase64;
    use std::sync::mpsc::{channel, Sender};
    use ::comms::{Controllable, CmdFrom};

    mod wrapper;

    /// Controllable struct for the camera
    pub struct Structure {
        /// Private handle to the device
        device: wrapper::Device,

        /// Private handle to the data stream
        depth: wrapper::VideoStream,

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
                wrapper::initialize();
                let device = wrapper::Device::new(None).unwrap();
                let depth = wrapper::VideoStream::new(&device, wrapper::OniSensorType::Depth).unwrap();
                println!("device = {:?}", device);
                println!("depth = {:?}", depth);
                depth.start();
                let start = time::now();
                let i = 0;
                Structure { tx: tx, device: device, depth: depth, start: start, i: i}
            }

            fn step(&mut self, _: Option<String>) -> bool {
                self.i += 1;

                let frame = prof!("readFrame", self.depth.readFrame().unwrap());
                let data: &[u8] = prof!(frame.data());

                let fname = format!("data/structure{}.png", self.i);
                let mut f = File::create(&fname).unwrap();
                let mut encoded = Vec::with_capacity(data.len());
                prof!("PNGEncoder", PNGEncoder::new(&mut encoded).encode(data, frame.width as u32, frame.height as u32, ColorType::Gray(16)));
                f.write_all(&encoded);

                prof!("tx.send", self.tx.send(CmdFrom::Data(format!("structure data:image/png;base64,{}", encoded.to_base64(base64::STANDARD)))));

                false
            }

            fn teardown(&mut self) {
                let end = time::now();
                self.depth.stop();
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

