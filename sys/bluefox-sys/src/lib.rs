#![allow(dead_code, non_camel_case_types)]

extern crate libc;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate macro_attr;
#[macro_use] extern crate conv;

use libc::{c_void, c_int, c_uint, c_char, c_double};
use std::ffi::CString;
use conv::TryFrom;
use std::slice;
use std::mem;
use std::ptr;

#[derive(Debug)]
pub enum MVError {
    Dmr(TDMR_ERROR),
    Prop(TPROPHANDLING_ERROR)
}

impl From<TDMR_ERROR> for MVError {
    fn from(err: TDMR_ERROR) -> MVError {
        MVError::Dmr(err)
    }
}

impl From<TPROPHANDLING_ERROR> for MVError {
    fn from(err: TPROPHANDLING_ERROR) -> MVError {
        MVError::Prop(err)
    }
}

macro_rules! dmr_status2result {
    ($code:expr) => { dmr_status2result!($code, ()) };
    ($code:expr, $ret:expr) => {
        match $code {
            TDMR_ERROR::DMR_NO_ERROR => Ok($ret),
            other => Err(TryFrom::try_from(other).unwrap_or(TDMR_ERROR::DMR_LAST_VALID_ERROR_CODE)),
        }
    }
}

macro_rules! prop_status2result {
    ($code:expr) => { prop_status2result!($code, ()) };
    ($code:expr, $ret:expr) => {
        match $code {
            TPROPHANDLING_ERROR::PROPHANDLING_NO_ERROR => Ok($ret),
            other => Err(TryFrom::try_from(other).unwrap_or(TPROPHANDLING_ERROR::PROPHANDLING_LAST_VALID_ERROR_CODE)),
        }
    }
}

macro_rules! c_str {
    ($s:expr) => {
        CString::new($s).unwrap().as_ptr()
    }
}

macro_rules! newtype {
    ($name:ident = $target:path) => {
        #[repr(C)]
        #[derive(Clone, Copy, Debug)]
        pub struct $name($target);
    }
}

newtype!(HDMR  = c_int);
newtype!(HDEV  = c_int);
newtype!(HDRV  = c_int);
newtype!(HOBJ  = c_int);
newtype!(HLIST = c_int);

impl HOBJ {
    fn into_hlist(self) -> HLIST {
        HLIST(self.0)
    }
}

macro_attr! {
    #[repr(C)]
    #[derive(Debug, TryFrom!(i32))]
    pub enum TDMR_ERROR {
        DMR_NO_ERROR                             = 0,
        DMR_DEV_NOT_FOUND                        = -2100,
        DMR_INIT_FAILED                          = -2101,
        DMR_DRV_ALREADY_IN_USE                   = -2102,
        DMR_DEV_CANNOT_OPEN                      = -2103,
        DMR_NOT_INITIALIZED                      = -2104,
        DMR_DRV_CANNOT_OPEN                      = -2105,
        DMR_DEV_REQUEST_QUEUE_EMPTY              = -2106,
        DMR_DEV_REQUEST_CREATION_FAILED          = -2107,
        DMR_INVALID_PARAMETER                    = -2108,
        DMR_EXPORTED_SYMBOL_NOT_FOUND            = -2109,
        DEV_UNKNOWN_ERROR                        = -2110,
        DEV_HANDLE_INVALID                       = -2111,
        DEV_INPUT_PARAM_INVALID                  = -2112,
        DEV_WRONG_INPUT_PARAM_COUNT              = -2113,
        DEV_CREATE_SETTING_FAILED                = -2114,
        DEV_REQUEST_CANT_BE_UNLOCKED             = -2115,
        DEV_INVALID_REQUEST_NUMBER               = -2116,
        DEV_LOCKED_REQUEST_IN_QUEUE              = -2117,
        DEV_NO_FREE_REQUEST_AVAILABLE            = -2118,
        DEV_WAIT_FOR_REQUEST_FAILED              = -2119,
        DEV_UNSUPPORTED_PARAMETER                = -2120,
        DEV_INVALID_RTC_NUMBER                   = -2121,
        DMR_INTERNAL_ERROR                       = -2122,
        DMR_INPUT_BUFFER_TOO_SMALL               = -2123,
        DEV_INTERNAL_ERROR                       = -2124,
        DMR_LIBRARY_NOT_FOUND                    = -2125,
        DMR_FUNCTION_NOT_IMPLEMENTED             = -2126,
        DMR_FEATURE_NOT_AVAILABLE                = -2127,
        DMR_EXECUTION_PROHIBITED                 = -2128,
        DMR_FILE_NOT_FOUND                       = -2129,
        DMR_INVALID_LICENCE                      = -2130,
        DEV_SENSOR_TYPE_ERROR                    = -2131,
        DMR_CAMERA_DESCRIPTION_INVALID           = -2132,
        DMR_NEWER_LIBRARY_REQUIRED               = -2133,
        DMR_TIMEOUT                              = -2134,
        DMR_WAIT_ABANDONED                       = -2135,
        DMR_EXECUTION_FAILED                     = -2136,
        DEV_REQUEST_ALREADY_IN_USE               = -2137,
        DEV_REQUEST_BUFFER_INVALID               = -2138,
        DEV_REQUEST_BUFFER_MISALIGNED            = -2139,
        DEV_ACCESS_DENIED                        = -2140,
        DMR_PRELOAD_CHECK_FAILED                 = -2141,
        DMR_CAMERA_DESCRIPTION_INVALID_PARAMETER = -2142,
        DMR_FILE_ACCESS_ERROR                    = -2143,
        DMR_INVALID_QUEUE_SELECTION              = -2144,
        DMR_ACQUISITION_ENGINE_BUSY              = -2145,
        //DMR_PSEUDO_LAST_ASSIGNED_ERROR_CODE,
        //DMR_LAST_ASSIGNED_ERROR_CODE           = DMR_PSEUDO_LAST_ASSIGNED_ERROR_CODE - 2,
        DMR_LAST_VALID_ERROR_CODE                = -2199,
    }
}

macro_attr! {
    #[repr(C)]
    #[derive(Debug, TryFrom!(i32))]
    pub enum TPROPHANDLING_ERROR {
        PROPHANDLING_NO_ERROR                           = 0,
        PROPHANDLING_NOT_A_LIST                         = -2000,
        PROPHANDLING_NOT_A_PROPERTY                     = -2001,
        PROPHANDLING_NOT_A_METHOD                       = -2002,
        PROPHANDLING_NO_READ_RIGHTS                     = -2003,
        PROPHANDLING_NO_WRITE_RIGHTS                    = -2004,
        PROPHANDLING_NO_MODIFY_SIZE_RIGHTS              = -2005,
        PROPHANDLING_INCOMPATIBLE_COMPONENTS            = -2006,
        PROPHANDLING_NO_USER_ALLOCATED_MEMORY           = -2007,
        PROPHANDLING_UNSUPPORTED_PARAMETER              = -2008,
        PROPHANDLING_SIZE_MISMATCH                      = -2009,
        PROPHANDLING_IMPLEMENTATION_MISSING             = -2010,
        PROPHANDLING_ACCESSTOKEN_CREATION_FAILED        = -2011,
        PROPHANDLING_INVALID_PROP_VALUE                 = -2012,
        PROPHANDLING_PROP_TRANSLATION_TABLE_CORRUPTED   = -2013,
        PROPHANDLING_PROP_VAL_ID_OUT_OF_BOUNDS          = -2014,
        PROPHANDLING_PROP_TRANSLATION_TABLE_NOT_DEFINED = -2015,
        PROPHANDLING_INVALID_PROP_VALUE_TYPE            = -2016,
        PROPHANDLING_PROP_VAL_TOO_LARGE                 = -2017,
        PROPHANDLING_PROP_VAL_TOO_SMALL                 = -2018,
        PROPHANDLING_COMPONENT_NOT_FOUND                = -2019,
        PROPHANDLING_LIST_ID_INVALID                    = -2020,
        PROPHANDLING_COMPONENT_ID_INVALID               = -2021,
        PROPHANDLING_LIST_ENTRY_OCCUPIED                = -2022,
        PROPHANDLING_COMPONENT_HAS_OWNER_ALREADY        = -2023,
        PROPHANDLING_COMPONENT_ALREADY_REGISTERED       = -2024,
        PROPHANDLING_LIST_CANT_ACCESS_DATA              = -2025,
        PROPHANDLING_METHOD_PTR_INVALID                 = -2026,
        PROPHANDLING_METHOD_INVALID_PARAM_LIST          = -2027,
        PROPHANDLING_SWIG_ERROR                         = -2028,
        PROPHANDLING_INVALID_INPUT_PARAMETER            = -2029,
        PROPHANDLING_COMPONENT_NO_CALLBACK_REGISTERED   = -2030,
        PROPHANDLING_INPUT_BUFFER_TOO_SMALL             = -2031,
        PROPHANDLING_WRONG_PARAM_COUNT                  = -2032,
        PROPHANDLING_UNSUPPORTED_OPERATION              = -2033,
        PROPHANDLING_CANT_SERIALIZE_DATA                = -2034,
        PROPHANDLING_INVALID_FILE_CONTENT               = -2035,
        PROPHANDLING_CANT_ALLOCATE_LIST                 = -2036,
        PROPHANDLING_CANT_REGISTER_COMPONENT            = -2037,
        PROPHANDLING_PROP_VALIDATION_FAILED             = -2038,
        //PROPHANDLING_PSEUDO_LAST_ASSIGNED_ERROR_CODE,
        //PROPHANDLING_LAST_ASSIGNED_ERROR_CODE         = PROPHANDLING_PSEUDO_LAST_ASSIGNED_ERROR_CODE - 2,
        PROPHANDLING_LAST_VALID_ERROR_CODE              = -2099,
    }
}

#[repr(C)]
#[derive(Debug)]
enum DeviceSearchMode {
    Serial   = 1,
    Family   = 2,
    Product  = 3,
    UseDevID = 0x8000,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub enum PixelFormat {
    Raw                  = 0,
    Mono8                = 1,
    Mono16               = 2,
    RGBx888Packed        = 3,
    YUV422Packed         = 4,
    RGBx888Planar        = 5,
    Mono10               = 6,
    Mono12               = 7,
    Mono14               = 8,
    RGB888Packed         = 9,
    YUV444Planar         = 10,
    Mono32               = 11,
    YUV422Planar         = 12,
    RGB101010Packed      = 13,
    RGB121212Packed      = 14,
    RGB141414Packed      = 15,
    RGB161616Packed      = 16,
    YUV422_UYVYPacked    = 17,
    Mono12Packed_V2      = 18,
    YUV422_10Packed      = 20,
    YUV422_UYVY_10Packed = 21,
    BGR888Packed         = 22,
    BGR101010Packed_V2   = 23,
    YUV444_UYVPacked     = 24,
    YUV444_UYV_10Packed  = 25,
    YUV444Packed         = 26,
    YUV444_10Packed      = 27,
    Mono12Packed_V1      = 28,
    Auto                 = -1,
    Unknown              = -2,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub enum ValueType {
    Int    = 0x1,
    Float  = 0x2,
    Ptr    = 0x3,
    String = 0x4,
    Int64  = 0x5,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub enum ComponentType {
    Prop = 0x00010000,
    List = 0x00020000,
    Meth = 0x00040000,
    PropInt    = ComponentType::Prop as isize | ValueType::Int as isize,
    PropFloat  = ComponentType::Prop as isize | ValueType::Float as isize,
    PropString = ComponentType::Prop as isize | ValueType::String as isize,
    PropPtr    = ComponentType::Prop as isize | ValueType::Ptr as isize,
    PropInt64  = ComponentType::Prop as isize | ValueType::Int64 as isize,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
enum ListType {
    Undefined              = -1,
    Setting                = 0,
    Request                = 1,
    RequestCtrl            = 2,
    Info                   = 3,
    Statistics             = 4,
    SystemSettings         = 5,
    IOSubSystem            = 6,
    RTCtr                  = 7,
    CameraDescriptions     = 8,
    DeviceSpecificData     = 9,
    EventSubSystemSettings = 10,
    EventSubSystemResults  = 11,
    ImageMemoryManager     = 12,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
enum SearchMode {
    IgnoreLists      = 0x2,
    IgnoreMethods    = 0x4,
    IgnoreProperties = 0x8,
}

#[repr(C, packed)]
pub struct ChannelData {
    pub channel_offset : c_int,
    pub line_pitch     : c_int,
    pub pixel_pitch    : c_int,
    pub channel_desc   : [c_char; 8192],
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct ImageBuffer {
    pub bytes_per_pixel : c_int,
    pub height          : c_int,
    pub width           : c_int,
    pub pixel_format    : PixelFormat,
    pub size            : c_int,
    pub data            : *mut c_void,
    pub channel_count   : c_int,
    pub channels        : *mut ChannelData,
}

pub struct Image<'a> {
    pub buf: ImageBuffer,
    reqnr: c_int,
    parent: &'a Device,
}

#[link(name = "mvPropHandling")]
extern "C" { }

#[link(name = "mvDeviceManager")]
extern "C" {
    // note: DMR_CALL = "" (on Linux)
    // note: MVDMR_API = __attribute__((visibility("default")))
    fn DMR_Init(pDevices: *mut HDMR) -> TDMR_ERROR;
    fn DMR_Close() -> TDMR_ERROR;

    fn DMR_GetDeviceCount(pDevCnt: *mut c_uint) -> TDMR_ERROR;
    fn DMR_GetDevice(pHDev: *mut HDEV, searchMode: DeviceSearchMode, pSearchString: *const c_char, devNr: c_uint, wildcard: c_char) -> TDMR_ERROR;
    fn DMR_OpenDevice(hDev: HDEV, pHDrv: *mut HDRV) -> TDMR_ERROR;
    fn DMR_CloseDevice(hDrv: HDRV, hDev: HDEV) -> TDMR_ERROR;

    fn DMR_ImageRequestSingle(hDrv: HDRV, requestCtrl: c_int, pRequestUsed: *mut c_int) -> TDMR_ERROR;
    fn DMR_ImageRequestWaitFor(hDrv: HDRV, timeout_ms: c_int, queueNr: c_int, pRequestNr: *mut c_int) -> TDMR_ERROR;
    fn DMR_ImageRequestUnlock(hDrv: HDRV, requestNr: c_int) -> TDMR_ERROR;
    fn DMR_GetImageRequestBuffer(hDrv: HDRV, requestNr: c_int, ppBuffer: *mut *mut ImageBuffer) -> TDMR_ERROR;
    fn DMR_ImageRequestReset(hDrv: HDRV, requestCtrl: c_int, mode: c_int) -> TDMR_ERROR;

    fn DMR_FindList(hDrv: HDRV, pName: *const c_char, typ: ListType, flags: c_uint, pHList: *mut HLIST) -> TDMR_ERROR;

    fn OBJ_GetHandleEx(hList: HLIST, pObjName: *const c_char, phObj: *mut HOBJ, searchMode: c_uint, maxSearchDepth: c_int) -> TPROPHANDLING_ERROR;
    fn OBJ_GetI(hProp: HOBJ, pVal: *mut c_int, index: c_int) -> TPROPHANDLING_ERROR;
    fn OBJ_SetI(hProp: HOBJ, val: c_int, index: c_int) -> TPROPHANDLING_ERROR;
    fn OBJ_GetI64(hProp: HOBJ, pVal: *mut i64, index: c_int) -> TPROPHANDLING_ERROR;
    fn OBJ_SetI64(hProp: HOBJ, val: i64, index: c_int) -> TPROPHANDLING_ERROR;
    fn OBJ_GetF(hProp: HOBJ, pVal: *mut c_double, index: c_int) -> TPROPHANDLING_ERROR;
    fn OBJ_SetF(hProp: HOBJ, val: c_double, index: c_int) -> TPROPHANDLING_ERROR;
    fn OBJ_GetType(hObj: HOBJ, pType: *mut ComponentType) -> TPROPHANDLING_ERROR;
}

pub struct Device {
    dmr: HDMR,
    dev: HDEV,
    drv: HDRV,
}

trait ObjProp {
    unsafe fn get(h_obj: HOBJ, pval: *mut Self, index: c_int) -> TPROPHANDLING_ERROR;
    unsafe fn set(h_obj: HOBJ, val: Self, index: c_int) -> TPROPHANDLING_ERROR;
}

macro_rules! obj_set_impl {
    ($t:ty, $get:ident, $set:ident) => {
        impl ObjProp for $t {
            unsafe fn get(h_obj: HOBJ, pval: *mut Self, index: c_int) -> TPROPHANDLING_ERROR { $get(h_obj, pval, index) }
            unsafe fn set(h_obj: HOBJ, val: Self, index: c_int) -> TPROPHANDLING_ERROR { $set(h_obj, val, index) }
        }
    }
}

obj_set_impl!(i32, OBJ_GetI,   OBJ_SetI);
obj_set_impl!(i64, OBJ_GetI64, OBJ_SetI64);
obj_set_impl!(f64, OBJ_GetF,   OBJ_SetF);

macro_rules! getter {
    ($name:ident, $list:expr, $prop:expr, $typ:ty) => {
        pub fn $name(&self) -> Result<$typ, MVError> {
            self.get_prop::<$typ>($list, $prop, 0)
        }
    };
    ($name:ident, $list:expr, $prop:expr, $typ:ty, |$conv_var:ident: $conv_typ:ty| $conv_body:expr) => {
        pub fn $name(&self) -> Result<$typ, MVError> {
            self.get_prop::<$conv_typ>($list, $prop, 0).map(|$conv_var| $conv_body)
        }
    }
}
macro_rules! setter {
    ($name:ident, $list:expr, $prop:expr, $typ:ty) => {
        pub fn $name(&self, val: $typ) -> Result<(), MVError> {
            self.set_prop::<$typ>($list, $prop, val, 0)
        }
    };
    ($name:ident, $list:expr, $prop:expr, $typ:ty, |$conv_var:ident: $conv_typ:ty| $conv_body:expr) => {
        pub fn $name(&self, $conv_var: $conv_typ) -> Result<(), MVError> {
            self.set_prop::<$typ>($list, $prop, $conv_body, 0)
        }
    }
}
macro_rules! getset {
    ($get:ident, $set:ident, $list:expr, $prop:expr, |$rvar:ident: $rty:ty| $r2c:expr, |$cvar:ident: $cty:ty| $c2r:expr) => {
        getter!($get, $list, $prop, $rty, |$cvar: $cty| $c2r);
        setter!($set, $list, $prop, $cty, |$rvar: $rty| $r2c);
    };
    ($get:ident, $set:ident, $list:expr, $prop:expr, $typ:ty) => {
        getter!($get, $list, $prop, $typ);
        setter!($set, $list, $prop, $typ);
    };
    ($get:ident, $set:ident, $list:expr, $prop:expr, $rty:ty as $cty:ty) => {
        getset!($get, $set, $list, $prop, |r: $rty| r as $cty, |c: $cty| TryFrom::try_from(c).unwrap());
    }
}

impl Device {
    pub fn new() -> Result<Device, TDMR_ERROR> {
        let mut this = Device { dmr: HDMR(0), dev: HDEV(0), drv: HDRV(0) };
        try!(dmr_status2result!(unsafe { DMR_Init(&mut this.dmr) }));
        let mut n: u32 = 0;
        try!(dmr_status2result!(unsafe { DMR_GetDeviceCount(&mut n as *mut _) }));
        println!("Have {} Bluefox devices.", n);
        try!(dmr_status2result!(unsafe { DMR_GetDevice(&mut this.dev,
                                                       DeviceSearchMode::Serial,
                                                       b"*\0" as *const u8 as *const c_char,
                                                       0,
                                                       b'*' as c_char) }));
        dmr_status2result!(unsafe { DMR_OpenDevice(this.dev,
                                                   &mut this.drv) }, this)
    }

    fn lookup_prop(&self, list: &str, prop: &str) -> Result<HOBJ, MVError> {
        let cstr_list = CString::new(list).unwrap();
        let cstr_prop = CString::new(prop).unwrap();
        unsafe {
            let mut base: HLIST = mem::uninitialized();
            let mut list: HOBJ = mem::uninitialized();
            let mut setting: HOBJ = mem::uninitialized();
            try!(dmr_status2result!(DMR_FindList(self.drv, b"Base\0" as *const [u8] as *const i8, ListType::Setting, 0, &mut base)));
            try!(prop_status2result!(OBJ_GetHandleEx(base, cstr_list.as_ptr(), &mut list, 0, -1)));
            try!(prop_status2result!(OBJ_GetHandleEx(list.into_hlist(), cstr_prop.as_ptr(), &mut setting, 0, -1)));
            Ok(setting)
        }
    }

    fn set_prop<T: ObjProp>(&self, list: &str, prop: &str, value: T, index: c_int) -> Result<(), MVError> {
        Ok(try!(prop_status2result!(unsafe { T::set(try!(self.lookup_prop(list, prop)), value, index) })))
    }

    fn get_prop<T: ObjProp>(&self, list: &str, prop: &str, index: c_int) -> Result<T, MVError> {
        unsafe {
            let mut value: T = mem::uninitialized();
            Ok(try!(prop_status2result!(T::get(try!(self.lookup_prop(list, prop)), &mut value, index), value)))
        }
    }

    pub fn request_reset(&self) -> Result<(), TDMR_ERROR> {
        dmr_status2result!(unsafe { DMR_ImageRequestReset(self.drv,
                                                          0,
                                                          0) })
    }

    pub fn request(&self) -> Result<Image, TDMR_ERROR> {
        try!(dmr_status2result!(unsafe { DMR_ImageRequestSingle(self.drv,
                                                                0,
                                                                ptr::null_mut()) }));
        let mut reqnr: c_int = 0;
        try!(dmr_status2result!(unsafe { DMR_ImageRequestWaitFor(self.drv,
                                                                 -1,
                                                                 0,
                                                                 &mut reqnr) }));
        let mut image_buf = ImageBuffer {
            bytes_per_pixel: 0,
            channel_count: 0,
            height: 0,
            size: 0,
            width: 0,
            channels: ptr::null_mut(),
            pixel_format: PixelFormat::Mono8,
            data: ptr::null_mut()
        };
        dmr_status2result!(unsafe { DMR_GetImageRequestBuffer(self.drv,
                                                              reqnr,
                                                              &mut &mut image_buf as *mut &mut ImageBuffer as *mut *mut ImageBuffer) },
                           Image {
                               buf: image_buf,
                               reqnr: reqnr,
                               parent: self
                           })
    }

    pub fn close(&self) -> Result<(), TDMR_ERROR> {
        try!(dmr_status2result!(unsafe { DMR_CloseDevice(self.drv,
                                                         self.dev) }));
        dmr_status2result!(unsafe { DMR_Close() })
    }
}

pub mod settings {
    use conv::TryFrom;
    use super::{Device, MVError};

    macro_rules! settings {
        ($(($name:ident: $typ:ty, $get:ident, $set:ident, $($rest:tt)*))*) => {
            #[derive(Debug, Default, Serialize, Deserialize)]
            pub struct Settings {
                $(
                    pub $name: Option<$typ>,
                )*
            }

            impl Device {
                $(
                    getset!($get, $set, $($rest)*);
                )*

                pub fn set(&mut self, s: &Settings) -> Result<(), MVError> {
                    $(
                        if let Some($name) = s.$name {
                            println!("BLUEFOX: set {} := {:?}", stringify!($name), $name);
                            self.$set($name)?;
                        }
                    )*
                    Ok(())
                }

                pub fn get(&self) -> Settings {
                    Settings {
                        $(
                            $name: match self.$get() {
                                Ok(v) => Some(v),
                                Err(e) => {
                                    println!("BLUEFOX: error getting {}: {:?}", stringify!($name), e);
                                    None
                                }
                            }
                        ),*
                    }
                }
            }
        }
    }

    settings! {
        (scale_enabled: bool,              get_scale_enabled, set_scale_enabled, "ImageDestination",   "ScalerMode",                   |b: bool| b as i32, |i: i32| i == 1)
        (scale_mode:    InterpolationMode, get_scale_mode,    set_scale_mode,    "ImageDestination",   "ScalerInterpolationMode",      InterpolationMode as i32           )
        (scale_width:   i32,               get_scale_width,   set_scale_width,   "ImageDestination",   "ImageWidth",                   i32                                )
        (scale_height:  i32,               get_scale_height,  set_scale_height,  "ImageDestination",   "ImageHeight",                  i32                                )
        (offset_x:      i64,               get_offset_x,      set_offset_x,      "ImageFormatControl", "OffsetX",                      i64                                )
        (offset_y:      i64,               get_offset_y,      set_offset_y,      "ImageFormatControl", "OffsetY",                      i64                                )
        (cam_format:    CameraPixelFormat, get_cam_format,    set_cam_format,    "ImageFormatControl", "PixelFormat",                  CameraPixelFormat as i64           )
        (dest_format:   DestPixelFormat,   get_dest_format,   set_dest_format,   "ImageDestination",   "PixelFormat",                  DestPixelFormat   as i32           )
        (bin_x:         i64,               get_bin_x,         set_bin_x,         "ImageFormatControl", "BinningHorizontal",            i64                                )
        (bin_y:         i64,               get_bin_y,         set_bin_y,         "ImageFormatControl", "BinningVertical",              i64                                )
        (decimate_x:    i64,               get_decimate_x,    set_decimate_x,    "ImageFormatControl", "DecimationHorizontal",         i64                                )
        (decimate_y:    i64,               get_decimate_y,    set_decimate_y,    "ImageFormatControl", "DecimationVertical",           i64                                )
        (width:         i64,               get_width,         set_width,         "ImageFormatControl", "Width",                        i64                                )
        (height:        i64,               get_height,        set_height,        "ImageFormatControl", "Height",                       i64                                )
        (reverse_x:     bool,              get_reverse_x,     set_reverse_x,     "ImageFormatControl", "ReverseX",                     |b: bool| b as i32, |i: i32| i == 1)
        (reverse_y:     bool,              get_reverse_y,     set_reverse_y,     "ImageFormatControl", "ReverseY",                     |b: bool| b as i32, |i: i32| i == 1)
        (acq_fr_enable: bool,              get_acq_fr_enable, set_acq_fr_enable, "AcquisitionControl", "mvAcquisitionFrameRateEnable", |b: bool| b as i32, |i: i32| i == 1)
        (acq_fr:        f64,               get_acq_fr,        set_acq_fr,        "AcquisitionControl", "AcquisitionFrameRate",         f64                                )
        (exposure_time: f64,               get_exposure_time, set_exposure_time, "AcquisitionControl", "ExposureTime",                 f64                                )
        (auto_exposure: bool,              get_auto_exposure, set_auto_exposure, "AcquisitionControl", "ExposureAuto",                 |b: bool| b as i32, |i: i32| i == 1)
        (auto_gain:     bool,              get_auto_gain,     set_auto_gain,     "AnalogControl",      "GainAuto",                     |b: bool| b as i32, |i: i32| i == 1)
        (white_balance: WhiteBalanceMode,  get_white_balance, set_white_balance, "AnalogControl",      "BalanceWhiteAuto",             WhiteBalanceMode  as i64           )
        (average_grey:  i64,               get_average_grey,  set_average_grey,  "AcquisitionControl", "mvExposureAutoAverageGrey",    i64                                )
    }

    macro_attr! {
        #[repr(C)]
        #[derive(Copy, Clone, Debug, TryFrom!(i64), Serialize, Deserialize)]
        pub enum CameraPixelFormat {
            BayerGR8      = 0x1080008,
            BayerGR10     = 0x110000C,
            BayerGR12     = 0x1100010,
            BayerGR16     = 0x110002E,
            RGB8Packed    = 0x2180014,
            BGR8Packed    = 0x2180015,
            BGRA8Packed   = 0x2200017,
            BGR10V2Packed = 0x220001D,
            RGB8          = 0,
            BGR8          = 1,
            BGRa8         = 2,
            RGB10p32      = 3,
        }
    }

    macro_attr! {
        #[repr(C)]
        #[derive(Copy, Clone, Debug, TryFrom!(i32), Serialize, Deserialize)]
        pub enum DestPixelFormat {
            Auto                 = 0,
            Raw                  = 1,
            Mono8                = 2,
            Mono10               = 6,
            Mono12               = 7,
            Mono12Packed_V1      = 28,
            Mono12Packed_V2      = 19,
            Mono14               = 8,
            Mono16               = 9,
            BGR888Packed         = 22,
            BGR101010Packed_V2   = 23,
            RGB888Packed         = 10,
            RGB101010Packed      = 14,
            RGB121212Packed      = 15,
            RGB141414Packed      = 16,
            RGB161616Packed      = 17,
            RGBx888Packed        = 3,
            RGBx888Planar        = 5,
            YUV422Packed         = 4,
            YUV422_UYVYPacked    = 18,
            YUV422_10Packed      = 20,
            YUV422_UYVY_10Packed = 21,
            YUV444_UYVPacked     = 24,
            YUV444_UYV_10Packed  = 25,
            YUV444Packed         = 26,
            YUV444_10Packed      = 27,
            YUV422Planar         = 13,
        }
    }

    macro_attr! {
        #[repr(C)]
        #[derive(Copy, Clone, Debug, TryFrom!(i32), Serialize, Deserialize)]
        pub enum InterpolationMode {
            NearestNeighbor = 0,
            Linear          = 1,
            Cubic           = 2,
        }
    }

    macro_attr! {
        #[repr(C)]
        #[derive(Copy, Clone, Debug, TryFrom!(i64), Serialize, Deserialize)]
        pub enum WhiteBalanceMode {
            Off        = 0,
            Once       = 1,
            Continuous = 2,
        }
    }
}

impl<'a> Image<'a> {
    pub fn size(&self) -> (usize, usize) {
        (self.buf.height as usize, self.buf.width as usize)
    }

    pub fn format(&self) -> PixelFormat {
        self.buf.pixel_format
    }

    pub fn data(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(mem::transmute(self.buf.data),
            self.buf.size as usize)
        }
    }
}

impl<'a> Drop for Image<'a> {
    fn drop(&mut self) {
        unsafe {
            DMR_ImageRequestUnlock(self.parent.drv, self.reqnr);
        }
    }
}
