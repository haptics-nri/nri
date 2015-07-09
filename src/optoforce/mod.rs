//! Service to read data from the OptoForce sensor
//!
//! # Optoforce drivers
//!
//! The driver SDK shipped by Optoforce is crap. The example barely compiles, depends on Qt (for
//! _serial port access_ of all things), and core dumps the first time it is run (after that it
//! freezes instead). The precompiled GUI application actually runs, which is a major step up, but
//! it was compiled with an absolute path to the fonts directory in someone's home directory, so it
//! requires a symlink under /home or an LD_PRELOAD library to display any characters. With that
//! minor annoyance fixed, it shows numbers and plots that confirm the sensor works! It can even
//! log values to a file, which may be useful for sanity checks later. But this isn't really good
//! enough for our use case, unless we did some terrible hack such as starting the GUI to write to
//! a file and having the NRI supervisor tail the file.
//!
//! Luckily, third-party drivers exist. A quick search found [liboptoforce][liboptoforce], out of
//! ETH Zurich, freely available on Github. I installed the software from their PPA. (The source is
//! in a git submodule.  Unfortunately building it requires ETH Zurich's own build system, called
//! ReMake, which I also checked out into a submodule, but I can't get that to build, so I can't
//! build liboptoforce either. Therefore, PPA it is.) I was able to compile the example program and
//! it shows numbers from the sensor!
//!
//! The ETH Zurich package includes a configuration program, which can set the sensor sample speed
//! (among other things), and a statistics program which can measure it (among other things).
//! Unfortunately, when to set to 1 kHz (the maximum) the sensor sends readings at only 500 Hz.
//! This is probably still enough for us. A calibration program is also included, which will be
//! useful for zeroing the sensor.
//!
//! - Commands to install liboptoforce:
//! <code>
//! $ sudo apt-add-repository ppa:ethz-asl/drivers
//! $ sudo apt-get update
//! $ sudo apt-get install liboptoforce*
//! </code>
//! - To compile and run the sample program:
//! <code>
//! $ sudo apt-get install libboost-{system,thread,signals,chrono,program-options}-dev
//! $ cd liboptoforce/src/bin
//! $ make
//! $ ./configure -d /dev/ttyACM0 -s 1000
//! $ ./statistics -d /dev/ttyACM0 -s
//! $ ./dump_readings -d /dev/ttyACM0 -s
//! $ ./calibrate -d /dev/ttyACM0
//! </code>
//!
//! [liboptoforce]: https://github.com/ethz-asl/liboptoforce

group_attr!{
    #[cfg(target_os = "linux")]

    use ::comms::{Controllable, CmdFrom, Block};
    use std::sync::mpsc::Sender;
    use std::default::Default;
    use std::thread;
    use std::io::Write;
    use std::fs::File;
    use std::{mem, slice, ptr};


    mod wrapper;

    pub struct Optoforce {
        device: wrapper::Device,
        i: usize,
        file: File
    }

    guilty!{
        impl Controllable for Optoforce {
            const NAME: &'static str = "optoforce",

            fn setup(_: Sender<CmdFrom>, _: Option<String>) -> Optoforce {
                let dev = wrapper::Device::new(Default::default());
                dev.connect(wrapper::ConnectOptions { path: "/dev/ttyOPTO", ..Default::default() });
                thread::sleep_ms(100);
                dev.set(wrapper::Settings::new()
                        .set_speed(wrapper::settings::Speed::Hz1000)
                       );
                println!("Optoforce settings: {:?}", dev.get().unwrap());
                Optoforce { device: dev, i: 0, file: File::create("data/optoforce.dat").unwrap() }
            }

            fn step(&mut self, _: Option<String>) -> Block {
                let xyz = self.device.read();
                self.file.write_all(unsafe { slice::from_raw_parts(&xyz as *const _ as *const _, mem::size_of_val(&xyz)) });
                self.i += 1;
                if (self.i % 100) == 0 {
                    println!("Optoforce #{} {:?}", self.i, xyz);
                }

                Block::Period(1000000)
            }

            fn teardown(&mut self) {
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
stub!(Optoforce);

