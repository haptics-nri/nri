//! Service to read data from the OptoForce sensor
//!
//! # Optoforce drivers
//!
//! The driver SDK shipped by Optoforce is crap. The example barely compiles, depends on Qt (for
//! _serial port access_ of all things), and core dumps the first time it is run (after that it
//! freezes instead). The precompiled GUI application actually runs, which is a major step up, but
//! it was compiled with an absolute path to the fonts directory in someone's home directory, so it
//! requires a symlink under /home or a LD_PRELOAD library to display any characters. With that
//! minor annoyance fixed, it shows numbers and plots that confirm the sensor works! It can even
//! log values to a file, which may be useful for sanity checks later. But this isn't really good
//! enough for our use case, unless we did some terrible hack such as starting the GUI to write to
//! a file and having the NRI supervisor tail the file.
//!
//! Luckily, third-party drivers exist. A quick search found liboptoforce, out of ETH Zurich,
//! freely available on Github. I installed the software from their PPA. (The source is in a git
//! submodule. Unfortunately building it requires ETH Zurich's own build system, called ReMake,
//! which I also checked out into a submodule, but I can't get that to build, so I can't build
//! liboptoforce either. Therefore, PPA it is.) I was able to compile the example program and it
//! shows numbers from the sensor! The Z axis seems to be centered at 30 N, so some calibration
//! will be required.
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
//! $ g++ dump_readings.cpp -std=c++11 -lboost_{program_options,system,chrono} -loptoforce
//! $ ./a.out -d /dev/ttyACM0 -s
//! </code>

use super::comms::{Controllable, CmdFrom};
use std::sync::mpsc::{channel, Sender};

stub!(Optoforce);


