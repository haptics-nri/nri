#[macro_use] extern crate lazy_static;
extern crate time;
extern crate notify;
extern crate libc;
extern crate errno;

#[macro_use] mod macros;
pub mod config;
mod extension_traits;
pub_use_mod!(fs);
pub_use_mod!(iter);
pub_use_mod!(misc);
pub mod prof;
pub use prof::PROF;

pub mod prelude {
    pub use super::config;
    pub use extension_traits::*;
}

