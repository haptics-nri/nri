//! This program exists to work around an apparent bug in the Bluefox drivers.
//! Namely, a given process may only set the settings ONCE.

#[macro_use] extern crate utils;

group_attr!{
    #[cfg(feature = "hardware")]

    extern crate libc;
    extern crate serde_json;
    extern crate bluefox_sys as ll;

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
}

#[cfg(not(feature = "hardware"))]
fn main() {
    println!("ERROR: this bin only runs on Proton hardware");
}

