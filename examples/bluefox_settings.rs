//! This program exists to work around an apparent bug in the Bluefox drivers.
//! Namely, a given process may only set the settings ONCE.

extern crate libc;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
#[macro_use] extern crate macro_attr;
#[macro_use] extern crate conv;

#[path="../crates/drivers/bluefox/src/wrapper.rs"]
mod ll;

use std::io;
use ll::Device;
use ll::settings::Settings;

fn main() {
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).unwrap();
    let settings: Settings = serde_json::from_str(&buffer).unwrap();

    let mut dev = Device::new().unwrap();
    println!("BEFORE:\n{:#?}\n", dev.get());
    dev.set(&settings).unwrap();
    println!("AFTER:\n{:#?}", dev.get());
}

