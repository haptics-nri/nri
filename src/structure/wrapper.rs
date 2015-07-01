extern crate libc;
use self::libc::{c_void, c_char, c_float, c_int};
use std::ptr;
use std::mem;
use std::ffi::CString;
use std::ops::Deref;
use std::slice;

#[repr(C)]
#[derive(PartialEq, Debug)]
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[derive(Debug)]
#[allow(dead_code)]
pub enum OniSensorType {
	IR    = 1,
	Color = 2,
	Depth = 3,

}

#[repr(C)]
#[derive(Debug,Copy,Clone)]
#[allow(dead_code)]
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

pub enum StreamProperty
{
	Cropping(OniCropping),
	HorizontalFOV(f32), // radians
	VerticalFOV(f32), // radians
	VideoMode(OniVideoMode),

	MaxValue(i32),
	MinValue(i32),

	Stride(i32),
	Mirroring(bool),

	NumberOfFrames(i32),

	// Camera
	AutoWhiteBalance(bool),
	AutoExposure(bool),
	Exposure(i32),
	Gain(i32),
}

impl Into<(c_int, *const c_void, c_int)> for StreamProperty {
    fn into(self) -> (c_int, *const c_void, c_int) {
        match self {
            StreamProperty::Cropping(cropping)    => (0,   &cropping            as *const _ as *const c_void, mem::size_of::<OniCropping>()  as c_int),
            StreamProperty::HorizontalFOV(fov)    => (1,   &(fov    as c_float) as *const _ as *const c_void, mem::size_of::<c_float>()      as c_int),
            StreamProperty::VerticalFOV(fov)      => (2,   &(fov    as c_float) as *const _ as *const c_void, mem::size_of::<c_float>()      as c_int),
            StreamProperty::VideoMode(mode)       => (3,   &mode                as *const _ as *const c_void, mem::size_of::<OniVideoMode>() as c_int),
            StreamProperty::MaxValue(max)         => (4,   &(max    as c_int)   as *const _ as *const c_void, mem::size_of::<c_int>()        as c_int),
            StreamProperty::MinValue(min)         => (5,   &(min    as c_int)   as *const _ as *const c_void, mem::size_of::<c_int>()        as c_int),
            StreamProperty::Stride(stride)        => (6,   &(stride as c_int)   as *const _ as *const c_void, mem::size_of::<c_int>()        as c_int),
            StreamProperty::Mirroring(mirror)     => (7,   &(mirror as c_int)   as *const _ as *const c_void, mem::size_of::<c_int>()        as c_int),
            StreamProperty::NumberOfFrames(nf)    => (8,   &(nf     as c_int)   as *const _ as *const c_void, mem::size_of::<c_int>()        as c_int),
            StreamProperty::AutoWhiteBalance(awb) => (100, &(awb    as c_int)   as *const _ as *const c_void, mem::size_of::<c_int>()        as c_int),
            StreamProperty::AutoExposure(ae)      => (101, &(ae     as c_int)   as *const _ as *const c_void, mem::size_of::<c_int>()        as c_int),
            StreamProperty::Exposure(exp)         => (102, &(exp    as c_int)   as *const _ as *const c_void, mem::size_of::<c_int>()        as c_int),
            StreamProperty::Gain(gain)            => (103, &(gain   as c_int)   as *const _ as *const c_void, mem::size_of::<c_int>()        as c_int),
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct OniCropping
{
	enabled: c_int,
	origin_x: c_int,
	origin_y: c_int,
	width: c_int,
	height: c_int,
}

#[repr(C)]
#[derive(Debug)]
#[allow(raw_pointer_derive)]
pub struct OniFrame {
	pub data_size: i32,
	data: *mut c_void,

	sensor_type: OniSensorType,
	timestamp: u64,
	frame_index: i32,

	pub width: i32,
	pub height: i32,

	video_mode: OniVideoMode,
	cropping_enabled: i32,
	crop_origin_x: i32,
	crop_origin_y: i32,

	stride: i32,
}

#[repr(C)]
#[derive(Debug,Copy,Clone)]
pub struct OniVideoMode {
	pub pixel_format: OniPixelFormat,
	pub resolution_x: i32,
	pub resolution_y: i32,
	pub fps: i32,
}

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
pub struct OniSensorInfo
{
	sensor_type: OniSensorType,
	num_supported_video_modes: i32,
	supported_video_modes: *const OniVideoMode
}

impl OniSensorInfo {
    pub fn video_modes(&self) -> &[OniVideoMode] {
        unsafe {
            slice::from_raw_parts(self.supported_video_modes, self.num_supported_video_modes as usize)
        }
    }
}

#[link(name = "OpenNI2")]
extern "C" {
    fn oniInitialize(apiVersion: i32) -> OniStatus;
    fn oniShutdown();

    fn oniDeviceOpen(uri: *const c_char, device: *mut *mut c_void) -> OniStatus;
    fn oniDeviceClose(device: *mut c_void) -> OniStatus;
    fn oniDeviceCreateStream(device: *mut c_void, sensorType: OniSensorType, pStream: *mut *mut c_void) -> OniStatus;

    // TODO typedefs for OniStreamHandle etc
    fn oniStreamStart(stream: *mut c_void) -> OniStatus;
    fn oniStreamReadFrame(stream: *mut c_void, pFrame: *mut *mut OniFrame) -> OniStatus;
    fn oniStreamStop(stream: *mut c_void);
    fn oniStreamDestroy(stream: *mut c_void);
    fn oniStreamGetSensorInfo(stream: *mut c_void) -> *const OniSensorInfo;
    fn oniStreamSetProperty(stream: *mut c_void, property_id: c_int, data: *const c_void, data_size: c_int) -> OniStatus;
    fn oniStreamGetProperty(stream: *mut c_void, property_id: c_int, data: *mut c_void, data_size: *mut c_int) -> OniStatus;

    fn oniFrameRelease(pFrame: *mut OniFrame);
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

#[allow(raw_pointer_derive)]
#[derive(Debug)]
pub struct Device {
    pdev: *mut c_void
}

#[allow(raw_pointer_derive)]
#[derive(Debug)]
pub struct VideoStream {
    pvs: *mut c_void
}

#[allow(raw_pointer_derive)]
#[derive(Debug)]
pub struct Frame {
    pf: *mut OniFrame
}

#[allow(raw_pointer_derive)]
#[derive(Debug)]
pub struct SensorInfo {
    pinfo: *const OniSensorInfo
}

macro_rules! c_str {
    ($s:expr) => {
        CString::new($s).unwrap().as_ptr()
    }
}

impl Device {
    pub fn new(uri: Option<&str>) -> Result<Device,OniError> {
        let mut dev = Device { pdev: ptr::null_mut() };
        let c_uri = match uri {
            Some(u) => c_str!(u),
            None    => ptr::null()
        };
        status2result!(unsafe { oniDeviceOpen(c_uri, &mut dev.pdev) }, dev)
    }

    pub fn close(&mut self) {
        unsafe { oniDeviceClose(self.pdev); } // TODO this returns an error which I am ignoring
    }
}

impl VideoStream {
    pub fn new(dev: &Device, sensor_type: OniSensorType) -> Result<VideoStream,OniError> {
        let mut vs = VideoStream { pvs: ptr::null_mut() };
        status2result!(unsafe { oniDeviceCreateStream(dev.pdev, sensor_type, &mut vs.pvs) }, vs)
    }

    pub fn start(&self) -> Result<(),OniError> {
        status2result!(unsafe { oniStreamStart(self.pvs) })
    }

    pub fn read_frame(&self) -> Result<Frame,OniError> {
        let mut pframe: *mut OniFrame = ptr::null_mut();
        try!(status2result!(unsafe { oniStreamReadFrame(self.pvs, &mut pframe) }));
        Ok(Frame { pf: unsafe { ptr::read(&pframe) } })
    }

    pub fn info(&self) -> Result<SensorInfo,OniError> {
        let pinfo = unsafe { oniStreamGetSensorInfo(self.pvs) };
        if pinfo.is_null() {
            Err(OniError::Error)
        } else {
            Ok(SensorInfo { pinfo: unsafe { ptr::read(&pinfo) } })
        }
    }

    pub fn set(&self, prop: StreamProperty) -> Result<(),OniError> {
        let converted_prop: (c_int, *const c_void, c_int) = prop.into();
        status2result!(unsafe { oniStreamSetProperty(self.pvs, converted_prop.0, converted_prop.1, converted_prop.2) })
    }

    pub fn get<X>(prop: &Fn(X) -> StreamProperty) -> Result<StreamProperty,OniError> {
        // FIXME need to redesign the StreamProperty type
        // so it can be used to construct the triples for set(), but also to get a number for get()
        // probably a guilty trait
    }

    pub fn stop(&self) {
        unsafe { oniStreamStop(self.pvs) }
    }
    
    pub fn destroy(&self) {
        unsafe { oniStreamDestroy(self.pvs) }
    }
}

impl Frame {
    pub fn data<T>(&self) -> &[T] {
        &unsafe { slice::from_raw_parts(self.data as *const T, self.data_size as usize) }
    }
}

impl Deref for Frame {
    type Target = OniFrame;

    fn deref(&self) -> &OniFrame {
        unsafe { &*self.pf }
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe { oniFrameRelease(self.pf) }
    }
}

impl Deref for SensorInfo {
    type Target = OniSensorInfo;

    fn deref(&self) -> &OniSensorInfo {
        unsafe { &*self.pinfo }
    }
}

