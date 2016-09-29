#![allow(dead_code)]

use libc::{c_void, c_char, c_int};
use conv::TryFrom;
use std::ptr;
use std::mem;
use std::ffi::{CString, CStr};
use std::cell::Cell;
use std::ops::Deref;
use std::slice;
//use std::time::Duration;

custom_derive! {
    #[repr(C)]
    #[derive(PartialEq, Debug, TryFrom(i32))]
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
}

impl OniStatus {
    fn into_err(self) -> OniError {
        OniError {
            code: unsafe { mem::transmute::<OniStatus, OniErrorCode>(self) },
            extended: unsafe { CStr::from_ptr(oniGetExtendedError()).to_str().unwrap().to_string() }
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct OniError {
    code: OniErrorCode,
    extended: String
}

#[repr(C)]
#[derive(Debug)]
pub enum OniErrorCode {
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
pub enum OniSensorType {
    IR    = 1,
    Color = 2,
    Depth = 3,

}

#[repr(C)]
#[derive(Debug,Copy,Clone)]
pub enum OniPixelFormat {
	// Depth
    Depth1mm   = 100,
    Depth100um = 101,
    Shift92    = 102,
    Shift93    = 103,

	// Color
    RGB888 = 200,
    YUV422 = 201,
    Gray8  = 202,
    Gray16 = 203,
    JPEG   = 204,
    YUYV   = 205,
}

pub mod prop {

    guilty! {
        pub trait Stream {
            const ID: i32,

            type Data;
            type CData;

            fn to_c(data: Self::Data) -> Self::CData;
            fn from_c(cdata: Self::CData) -> Self::Data;
        }
    }

    macro_rules! stream_property_impl {
        // multiple names with the same data types (+ short forms)
        ([$($name:ident = $id:expr),*],
         $data:ty) => {
             $(
                 stream_property_impl!($name = $id,
                                       $data);
              )*
         };
        ([$($name:ident = $id:expr),*],
         $data:ty => $cdata:ty) => {
             $(
                 stream_property_impl!($name = $id,
                                       $data => $cdata);
              )*
         };
        ([$($name:ident = $id:expr),*],
         $data:ty => $cdata:ty,
         |$to_param:ident| $to_body:expr,
         |$from_param:ident| $from_body:expr) => {
             $(
                 stream_property_impl!($name = $id,
                                       $data => $cdata,
                                       |$to_param| $to_body,
                                       |$from_param| $from_body);
              )*
         };

        // short form: converters default to straight casts
        ($name:ident = $id:expr,
         $data:ty => $cdata:ty) => {
             stream_property_impl!($name = $id,
                                   $data => $cdata,
                                   |d| d as $cdata,
                                   |cd| cd as $data);
         };

        // shorter form: $cdata defaults to $data (and therefore the converters are no-ops)
        ($name:ident = $id:expr,
         $data:ty) => {
             stream_property_impl!($name = $id, $data => $data, |d| d, |cd| cd);
         };

        ($name:ident = $id:expr,
         $data:ty => $cdata:ty,
         |$to_param:ident| $to_body:expr,
         |$from_param:ident| $from_body:expr) => {
            pub struct $name;
            guilty! {
                impl Stream for $name {
                    const ID: i32 = $id,

                    type Data = $data;
                    type CData = $cdata;

                    fn to_c($to_param: $data) -> $cdata { $to_body }
                    fn from_c($from_param: $cdata) -> $data { $from_body }
                }
            }
        };
    }

    use super::{OniCropping, OniVideoMode};
    use libc::{c_int, c_float};

    pub type Radians = f32;

    stream_property_impl!(Cropping = 0, OniCropping);
    stream_property_impl!(VideoMode = 3, OniVideoMode);
    stream_property_impl!([HorizontalFOV = 1, VerticalFOV = 2], Radians => c_float);
    stream_property_impl!([MaxValue = 4, MinValue = 5, Stride = 6, NumberOfFrames = 8, Exposure = 102, Gain = 103], i32 => c_int);
    stream_property_impl!([Mirroring = 7, AutoWhiteBalance = 100, AutoExposure = 101], bool => c_int, |b| b as c_int, |i| i != 0);

}

#[repr(C)]
#[derive(Debug)]
pub struct OniCropping {
    enabled  : c_int,
    origin_x : c_int,
    origin_y : c_int,
    width    : c_int,
    height   : c_int,
}

#[repr(C)]
#[derive(Debug)]
pub struct OniFrame {
    pub data_size    : i32,
    data             : *mut c_void,

    sensor_type      : OniSensorType,
    timestamp        : u64,
    frame_index      : i32,

    pub width        : i32,
    pub height       : i32,

    video_mode       : OniVideoMode,
    cropping_enabled : i32,
    crop_origin_x    : i32,
    crop_origin_y    : i32,

    stride           : i32,
}

#[repr(C)]
#[derive(Debug,Copy,Clone)]
pub struct OniVideoMode {
    pub pixel_format : OniPixelFormat,
    pub resolution_x : i32,
    pub resolution_y : i32,
    pub fps          : i32,
}

#[repr(C)]
#[derive(Debug)]
pub struct OniSensorInfo {
    sensor_type               : OniSensorType,
    num_supported_video_modes : i32,
    supported_video_modes     : *const OniVideoMode,
}

impl OniSensorInfo {
    pub fn video_modes(&self) -> &[OniVideoMode] {
        unsafe { slice::from_raw_parts(self.supported_video_modes,
                                       self.num_supported_video_modes as usize) }
    }
}

#[link(name = "OpenNI2")]
extern "C" {
    fn oniInitialize(apiVersion: i32) -> OniStatus;
    fn oniShutdown();
    fn oniGetExtendedError() -> *const c_char;

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
            other => Err(TryFrom::try_from(other).unwrap_or(OniStatus::Error).into_err())
        }
    }
}

pub fn initialize() -> Result<(), OniError> {
    /* TODO it'd be real nice if we could reset it without unplugging it
    match ioctl::usbdevfs_reset("/dev/bus/usb/002/002") {
        Ok(_) => println!("Device reset succeeded"),
        Err(rc) => {
            println!("Device reset failed: {}", rc);
            return Err(OniError::Error);
        },
    }
    thread::sleep(Duration::from_millis(5000));
    */
    status2result!(unsafe { oniInitialize(2) })
}

pub fn shutdown() {
    unsafe { oniShutdown() }
}

#[derive(Debug)]
pub struct Device {
    pdev: *mut c_void,
}

#[derive(Debug)]
pub struct VideoStream {
    pvs: *mut c_void,
    running: Cell<bool>,
}

#[derive(Debug)]
pub struct Frame {
    pf: *mut OniFrame,
}

#[derive(Debug)]
pub struct SensorInfo {
    pinfo: *const OniSensorInfo,
}

macro_rules! c_str {
    ($s:expr) => {
        CString::new($s).unwrap().as_ptr()
    }
}

impl Device {
    pub fn new(uri: Option<&str>) -> Result<Device, OniError> {
        let mut dev = Device { pdev: ptr::null_mut() };
        let c_uri = match uri {
            Some(u) => c_str!(u),
            None    => ptr::null(),
        };
        status2result!(unsafe { oniDeviceOpen(c_uri, &mut dev.pdev) }, dev)
    }

    pub fn close(&mut self) {
        unsafe {
            oniDeviceClose(self.pdev);
        } // TODO this returns an error which I am ignoring
    }
}

impl VideoStream {
    pub fn new(dev: &Device, sensor_type: OniSensorType) -> Result<VideoStream, OniError> {
        let mut vs = VideoStream { pvs: ptr::null_mut(), running: Cell::new(false) };
        status2result!(unsafe { oniDeviceCreateStream(dev.pdev, sensor_type, &mut vs.pvs) }, vs)
    }

    pub fn start(&self) -> Result<(), OniError> {
        try!(status2result!(unsafe { oniStreamStart(self.pvs) }));
        self.running.set(true);
        Ok(())
    }

    pub fn stop(&self) {
        unsafe { oniStreamStop(self.pvs) };
        self.running.set(false);
    }

    pub fn is_running(&self) -> bool {
        self.running.get()
    }

    pub fn read_frame(&self) -> Result<Frame, OniError> {
        let mut pframe: *mut OniFrame = ptr::null_mut();
        try!(status2result!(unsafe { oniStreamReadFrame(self.pvs, &mut pframe) }));
        Ok(Frame { pf: unsafe { ptr::read(&pframe) } })
    }

    pub fn info(&self) -> Result<SensorInfo, OniError> {
        let pinfo = unsafe { oniStreamGetSensorInfo(self.pvs) };
        if pinfo.is_null() {
            Err(OniStatus::Error.into_err())
        } else {
            Ok(SensorInfo { pinfo: unsafe { ptr::read(&pinfo) } })
        }
    }

    pub fn set<P: prop::Stream>(&self, data: P::Data) -> Result<(), OniError> {
        status2result!(unsafe { oniStreamSetProperty(self.pvs, guilty!(P::ID), &P::to_c(data) as *const _ as *const c_void, mem::size_of::<P::CData>() as c_int) })
    }

    pub fn get<P: prop::Stream>(&self) -> Result<P::Data, OniError> {
        unsafe {
            let mut cdata: P::CData = mem::uninitialized();
            let mut size : c_int    = mem::uninitialized();
            try!(status2result!(oniStreamGetProperty(self.pvs, guilty!(P::ID), &mut cdata as *mut _ as *mut c_void, &mut size)));
            assert_eq!(size as usize, mem::size_of::<P::CData>());
            Ok(P::from_c(cdata))
        }
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
