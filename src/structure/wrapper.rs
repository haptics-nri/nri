extern crate libc;
use libc::{c_void, c_char};
use std::ptr;
use std::mem;
use std::ffi::CString;

#[repr(C)]
#[derive(PartialEq, Debug)]
enum OniStatus {
	Ok             = 0,
	Error          = 1,
	NotImplemented = 2,
	NotSupported   = 3,
	BadParameter   = 4,
	OutOfFlow      = 5,
	NoDevice       = 6,
	TimeOut        = 102,
}

#[repr(C)]
#[derive(Debug)]
pub enum OniError {
	Error          = 1,
	NotImplemented = 2,
	NotSupported   = 3,
	BadParameter   = 4,
	OutOfFlow      = 5,
	NoDevice       = 6,
	TimeOut        = 102,
}

#[repr(C)]
pub enum OniSensorType {
	IR    = 1,
	Color = 2,
	Depth = 3,

}

#[repr(C)]
pub enum OniPixelFormat {
	// Depth
	Depth1mm   = 100,
	Depth100um = 101,
	Shift92    = 102,
	Shift93    = 103,

	// Color
	RGB888     = 200,
	YUV422     = 201,
	Gray8      = 202,
	Gray16     = 203,
	JPEG       = 204,
	YUYV       = 205,
}

#[repr(C)]
pub struct OniFrame {
	dataSize: i32,
	data: *mut c_void,

	sensorType: OniSensorType,
	timestamp: u64,
	frameIndex: i32,

	width: i32,
	height: i32,

	videoMode: OniVideoMode,
	croppingEnabled: i32,
	cropOriginX: i32,
	cropOriginY: i32,

	stride: i32,
}

#[repr(C)]
pub struct OniVideoMode {
	pixelFormat: OniPixelFormat,
	resolutionX: i32,
	resolutionY: i32,
	fps: i32,
}

#[link(name = "OpenNI2")]
extern "C" {
    fn oniInitialize(apiVersion: i32) -> OniStatus;
    fn oniShutdown();
    fn oniDeviceOpen(uri: *const c_char, device: *mut *mut c_void) -> OniStatus;
    fn oniDeviceClose(device: *mut c_void) -> OniStatus;
    fn oniDeviceCreateStream(device: *mut c_void, sensorType: OniSensorType, pStream: *mut *mut c_void) -> OniStatus;

    fn oniStreamStart(stream: *mut c_void) -> OniStatus;
    fn oniStreamReadFrame(stream: *mut c_void, pFrame: *mut *mut c_void) -> OniStatus;
    fn oniStreamStop(stream: *mut c_void);
    fn oniStreamDestroy(stream: *mut c_void);

    pub static ANY_DEVICE: &'static str;
}

macro_rules! status2result {
    ($code:expr) => { status2result!($code, ()) };
    ($code:expr, $ret:expr) => {
        match $code {
            OniStatus::Ok => Ok($ret),
            other => Err(unsafe { mem::transmute(other) }) // TODO make this safe
        }
    }
}

pub fn initialize() -> Result<(),OniError> {
    status2result!(unsafe { oniInitialize(2) })
}

pub fn shutdown() {
    unsafe { oniShutdown() }
}

pub struct Device {
    pdev: *mut libc::c_void
}

pub struct VideoStream {
    pvs: *mut libc::c_void
}

macro_rules! c_str {
    ($s:expr) => {
        CString::new($s).unwrap().as_ptr()
    }
}

impl Device {
    pub fn new(uri: &str) -> Result<Device,OniError> {
        let mut dev = Device { pdev: ptr::null_mut() };
        status2result!(unsafe { oniDeviceOpen(c_str!(uri), &mut dev.pdev) }, dev)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        assert_eq!(unsafe { oniDeviceClose(self.pdev) }, OniStatus::Ok)
    }
}

impl VideoStream {
    pub fn new(dev: Device, sensor_type: OniSensorType) -> Result<VideoStream,OniError> {
        let mut vs = VideoStream { pvs: ptr::null_mut() };
        status2result!(unsafe { oniDeviceCreateStream(dev.pdev, sensor_type, &mut vs.pvs) }, vs)
    }
    pub fn start(&self) -> Result<(),OniError> {
        status2result!(unsafe { oniStreamStart(self.pvs) })
    }
    pub fn readFrame(&self) -> Result<Vec<u8>,OniError> {
        let mut pframe = ptr::null_mut();
        try!(status2result!(unsafe { oniStreamReadFrame(self.pvs, &mut pframe) }));

        Err(OniError::NotImplemented)
    }
    pub fn stop(&self) {
        unsafe { oniStreamStop(self.pvs) }
    }
    pub fn destroy(&self) {
        unsafe { oniStreamDestroy(self.pvs) }
    }
}

