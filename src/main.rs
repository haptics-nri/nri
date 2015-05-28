mod structure;
mod bluefox;
mod optoforce;

use structure::wrapper::Allocated;

#[macro_use]
extern crate log;
extern crate env_logger;

fn main() {
    env_logger::init().unwrap();

    info!("Hello, world!");

    let dev = structure::wrapper::Device::new();
}
