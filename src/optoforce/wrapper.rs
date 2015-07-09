extern crate libc;
use self::libc::{c_void, c_int, c_char, c_float, c_double};
use std::default::Default;
use std::f32;
use std::ffi::CString;

macro_rules! c_str {
    ($s:expr) => {
        CString::new($s).unwrap().as_ptr()
    }
}


#[link(name = "optoforce_adapter")]
extern "C" {
    fn lofa_get_version() -> Version;
    fn lofa_create_sensor(buffer: c_int, factor: c_float) -> Handle;
    fn lofa_sensor_connect(that: Handle, device: *const c_char, baudrate: c_int) -> bool;
    fn lofa_sensor_disconnect(that: Handle, should_block: bool);
    fn lofa_free_sensor(that: Handle);
    fn lofa_sensor_read(that: Handle) -> XYZ;
}

#[repr(C)]
pub struct Version {
    major:    c_int,
    minor:    c_int,
    revision: c_int
}

#[derive(Debug)]
#[repr(C)]
pub struct XYZ {
    x: c_double,
    y: c_double,
    z: c_double
}

type Handle = *mut c_void;

pub struct Device {
    pdev: Handle
}

impl Device {
    pub fn new(opt: DeviceOptions) -> Device {
        Device { pdev: unsafe { lofa_create_sensor(opt.buffer as c_int, opt.factor as c_float) } }
    }

    pub fn connect(&self, dev: &str, baud: i32) -> Result<(),()> {
        match unsafe { lofa_sensor_connect(self.pdev, c_str!(dev), baud) } {
            true => Ok(()),
            false => Err(())
        }
    }

    pub fn disconnect(&self, block: bool) {
        unsafe { lofa_sensor_disconnect(self.pdev, block); }
    }

    pub fn read(&self) -> XYZ {
        unsafe { lofa_sensor_read(self.pdev) }
    }
}

pub struct DeviceOptions {
    buffer: isize,
    factor: f32
}

impl Default for DeviceOptions {
    fn default() -> DeviceOptions {
        DeviceOptions { buffer: -1, factor: f32::NAN }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            lofa_sensor_disconnect(self.pdev, false);
            lofa_free_sensor(self.pdev);
        }
    }
}

