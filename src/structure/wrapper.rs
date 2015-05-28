extern crate libc;

#[link(name = "OpenNI2_adapter")]
extern "C"
{
    fn Device_new() -> *const libc::c_void;
    fn Device_delete(that: *const libc::c_void);
    fn VideoStream_new() -> *const libc::c_void;
    fn VideoStream_delete(that: *const libc::c_void);
}

pub struct Device {
    this: *const libc::c_void
}

pub struct VideoStream {
    this: *const libc::c_void
}

pub trait Allocated<T> : Drop {
    fn new() -> T;
}

impl Allocated<Device> for Device {
    fn new() -> Device {
        Device { this: unsafe { Device_new() } }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { Device_delete(self.this) }
    }
}

impl Allocated<VideoStream> for VideoStream {
    fn new() -> VideoStream {
        VideoStream { this: unsafe { VideoStream_new() } }
    }
}

impl Drop for VideoStream {
    fn drop(&mut self) {
        unsafe { VideoStream_delete(self.this) }
    }
}

