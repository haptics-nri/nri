extern crate libc;
use self::libc::{c_void, c_int, c_uint, c_char};
use std::slice;
use std::mem;
use std::ptr;
use std::ffi::CString;

#[repr(C)] #[derive(Clone, Copy)] pub struct HDMR(c_int);
#[repr(C)] #[derive(Clone, Copy)] pub struct HDEV(c_int);
#[repr(C)] #[derive(Clone, Copy)] pub struct HDRV(c_int);
#[repr(C)] #[derive(Clone, Copy)] pub struct HOBJ(c_int);
#[repr(C)] #[derive(Clone, Copy)] pub struct HLIST(c_int);

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum TDMR_ERROR {
    DMR_NO_ERROR = 0,
    DMR_DEV_NOT_FOUND = -2100,
    DMR_INIT_FAILED = -2101,
    DMR_DRV_ALREADY_IN_USE = -2102,
    DMR_DEV_CANNOT_OPEN = -2103,
    DMR_NOT_INITIALIZED = -2104,
    DMR_DRV_CANNOT_OPEN = -2105,
    DMR_DEV_REQUEST_QUEUE_EMPTY = -2106,
    DMR_DEV_REQUEST_CREATION_FAILED = -2107,
    DMR_INVALID_PARAMETER = -2108,
    DMR_EXPORTED_SYMBOL_NOT_FOUND = -2109,
    DEV_UNKNOWN_ERROR = -2110,
    DEV_HANDLE_INVALID = -2111,
    DEV_INPUT_PARAM_INVALID = -2112,
    DEV_WRONG_INPUT_PARAM_COUNT = -2113,
    DEV_CREATE_SETTING_FAILED = -2114,
    DEV_REQUEST_CANT_BE_UNLOCKED = -2115,
    DEV_INVALID_REQUEST_NUMBER = -2116,
    DEV_LOCKED_REQUEST_IN_QUEUE = -2117,
    DEV_NO_FREE_REQUEST_AVAILABLE = -2118,
    DEV_WAIT_FOR_REQUEST_FAILED = -2119,
    DEV_UNSUPPORTED_PARAMETER = -2120,
    DEV_INVALID_RTC_NUMBER = -2121,
    DMR_INTERNAL_ERROR = -2122,
    DMR_INPUT_BUFFER_TOO_SMALL = -2123,
    DEV_INTERNAL_ERROR = -2124,
    DMR_LIBRARY_NOT_FOUND = -2125,
    DMR_FUNCTION_NOT_IMPLEMENTED = -2126,
    DMR_FEATURE_NOT_AVAILABLE = -2127,
    DMR_EXECUTION_PROHIBITED = -2128,
    DMR_FILE_NOT_FOUND = -2129,
    DMR_INVALID_LICENCE = -2130,
    DEV_SENSOR_TYPE_ERROR = -2131,
    DMR_CAMERA_DESCRIPTION_INVALID = -2132,
    DMR_NEWER_LIBRARY_REQUIRED = -2133,
    DMR_TIMEOUT = -2134,
    DMR_WAIT_ABANDONED = -2135,
    DMR_EXECUTION_FAILED = -2136,
    DEV_REQUEST_ALREADY_IN_USE = -2137,
    DEV_REQUEST_BUFFER_INVALID = -2138,
    DEV_REQUEST_BUFFER_MISALIGNED = -2139,
    DEV_ACCESS_DENIED = -2140,
    DMR_PRELOAD_CHECK_FAILED = -2141,
    DMR_CAMERA_DESCRIPTION_INVALID_PARAMETER = -2142,
    DMR_FILE_ACCESS_ERROR = -2143,
    DMR_INVALID_QUEUE_SELECTION = -2144,
    DMR_ACQUISITION_ENGINE_BUSY = -2145,
    //DMR_PSEUDO_LAST_ASSIGNED_ERROR_CODE,
    //DMR_LAST_ASSIGNED_ERROR_CODE = DMR_PSEUDO_LAST_ASSIGNED_ERROR_CODE - 2,
    DMR_LAST_VALID_ERROR_CODE = -2199
}

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum TPROPHANDLING_ERROR {
    PROPHANDLING_NO_ERROR = 0,
    PROPHANDLING_NOT_A_LIST = -2000,
    PROPHANDLING_NOT_A_PROPERTY = -2001,
    PROPHANDLING_NOT_A_METHOD = -2002,
    PROPHANDLING_NO_READ_RIGHTS = -2003,
    PROPHANDLING_NO_WRITE_RIGHTS = -2004,
    PROPHANDLING_NO_MODIFY_SIZE_RIGHTS = -2005,
    PROPHANDLING_INCOMPATIBLE_COMPONENTS = -2006,
    PROPHANDLING_NO_USER_ALLOCATED_MEMORY = -2007,
    PROPHANDLING_UNSUPPORTED_PARAMETER = -2008,
    PROPHANDLING_SIZE_MISMATCH = -2009,
    PROPHANDLING_IMPLEMENTATION_MISSING = -2010,
    PROPHANDLING_ACCESSTOKEN_CREATION_FAILED = -2011,
    PROPHANDLING_INVALID_PROP_VALUE = -2012,
    PROPHANDLING_PROP_TRANSLATION_TABLE_CORRUPTED = -2013,
    PROPHANDLING_PROP_VAL_ID_OUT_OF_BOUNDS = -2014,
    PROPHANDLING_PROP_TRANSLATION_TABLE_NOT_DEFINED = -2015,
    PROPHANDLING_INVALID_PROP_VALUE_TYPE = -2016,
    PROPHANDLING_PROP_VAL_TOO_LARGE = -2017,
    PROPHANDLING_PROP_VAL_TOO_SMALL = -2018,
    PROPHANDLING_COMPONENT_NOT_FOUND = -2019,
    PROPHANDLING_LIST_ID_INVALID = -2020,
    PROPHANDLING_COMPONENT_ID_INVALID = -2021,
    PROPHANDLING_LIST_ENTRY_OCCUPIED = -2022,
    PROPHANDLING_COMPONENT_HAS_OWNER_ALREADY = -2023,
    PROPHANDLING_COMPONENT_ALREADY_REGISTERED = -2024,
    PROPHANDLING_LIST_CANT_ACCESS_DATA = -2025,
    PROPHANDLING_METHOD_PTR_INVALID = -2026,
    PROPHANDLING_METHOD_INVALID_PARAM_LIST = -2027,
    PROPHANDLING_SWIG_ERROR = -2028,
    PROPHANDLING_INVALID_INPUT_PARAMETER = -2029,
    PROPHANDLING_COMPONENT_NO_CALLBACK_REGISTERED = -2030,
    PROPHANDLING_INPUT_BUFFER_TOO_SMALL = -2031,
    PROPHANDLING_WRONG_PARAM_COUNT = -2032,
    PROPHANDLING_UNSUPPORTED_OPERATION = -2033,
    PROPHANDLING_CANT_SERIALIZE_DATA = -2034,
    PROPHANDLING_INVALID_FILE_CONTENT = -2035,
    PROPHANDLING_CANT_ALLOCATE_LIST = -2036,
    PROPHANDLING_CANT_REGISTER_COMPONENT = -2037,
    PROPHANDLING_PROP_VALIDATION_FAILED = -2038,
    //PROPHANDLING_PSEUDO_LAST_ASSIGNED_ERROR_CODE,
    //PROPHANDLING_LAST_ASSIGNED_ERROR_CODE = PROPHANDLING_PSEUDO_LAST_ASSIGNED_ERROR_CODE - 2,
    PROPHANDLING_LAST_VALID_ERROR_CODE = -2099
}

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
enum DeviceSearchMode {
    Serial     = 1,
    Family     = 2,
    Product    = 3,
    UseDevID   = 0x8000
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub enum PixelFormat {
    Raw = 0,
    Mono8 = 1,
    Mono16 = 2,
    RGBx888Packed = 3,
    YUV422Packed = 4,
    RGBx888Planar = 5,
    Mono10 = 6,
    Mono12 = 7,
    Mono14 = 8,
    RGB888Packed = 9,
    YUV444Planar = 10,
    Mono32 = 11,
    YUV422Planar = 12,
    RGB101010Packed = 13,
    RGB121212Packed = 14,
    RGB141414Packed = 15,
    RGB161616Packed = 16,
    YUV422_UYVYPacked = 17,
    Mono12Packed_V2 = 18,
    YUV422_10Packed = 20,
    YUV422_UYVY_10Packed = 21,
    BGR888Packed = 22,
    BGR101010Packed_V2 = 23,
    YUV444_UYVPacked = 24,
    YUV444_UYV_10Packed = 25,
    YUV444Packed = 26,
    YUV444_10Packed = 27,
    Mono12Packed_V1 = 28,
    Auto = -1
}

impl PixelFormat {
    fn to_prop(px: PixelFormat) -> Result<i32,PixelFormat> {
        match px {
            PixelFormat::Auto => Ok(0),
            PixelFormat::Raw => Ok(1),
            PixelFormat::Mono8 => Ok(2),
            PixelFormat::Mono10 => Ok(6),
            PixelFormat::Mono12 => Ok(7),
            PixelFormat::Mono12Packed_V1 => Ok(28),
            PixelFormat::Mono12Packed_V2 => Ok(19),
            PixelFormat::Mono14 => Ok(8),
            PixelFormat::Mono16 => Ok(9),
            PixelFormat::BGR888Packed => Ok(22),
            PixelFormat::BGR101010Packed_V2 => Ok(23),
            PixelFormat::RGB888Packed => Ok(10),
            PixelFormat::RGB101010Packed => Ok(14),
            PixelFormat::RGB121212Packed => Ok(15),
            PixelFormat::RGB141414Packed => Ok(16),
            PixelFormat::RGB161616Packed => Ok(17),
            PixelFormat::RGBx888Packed => Ok(3),
            PixelFormat::RGBx888Planar => Ok(5),
            PixelFormat::YUV422Packed => Ok(4),
            PixelFormat::YUV422_UYVYPacked => Ok(18),
            PixelFormat::YUV422_10Packed => Ok(20),
            PixelFormat::YUV422_UYVY_10Packed => Ok(21),
            PixelFormat::YUV444_UYVPacked => Ok(24),
            PixelFormat::YUV444_UYV_10Packed => Ok(25),
            PixelFormat::YUV444Packed => Ok(26),
            PixelFormat::YUV444_10Packed => Ok(27),
            PixelFormat::YUV422Planar => Ok(13),
            _ => Err(px)
        }
    }

    fn from_prop(i: i32) -> Result<PixelFormat,i32> {
        match i {
            0 => Ok(PixelFormat::Auto),
            1 => Ok(PixelFormat::Raw),
            2 => Ok(PixelFormat::Mono8),
            6 => Ok(PixelFormat::Mono10),
            7 => Ok(PixelFormat::Mono12),
            28 => Ok(PixelFormat::Mono12Packed_V1),
            19 => Ok(PixelFormat::Mono12Packed_V2),
            8 => Ok(PixelFormat::Mono14),
            9 => Ok(PixelFormat::Mono16),
            22 => Ok(PixelFormat::BGR888Packed),
            23 => Ok(PixelFormat::BGR101010Packed_V2),
            10 => Ok(PixelFormat::RGB888Packed),
            14 => Ok(PixelFormat::RGB101010Packed),
            15 => Ok(PixelFormat::RGB121212Packed),
            16 => Ok(PixelFormat::RGB141414Packed),
            17 => Ok(PixelFormat::RGB161616Packed),
            3 => Ok(PixelFormat::RGBx888Packed),
            5 => Ok(PixelFormat::RGBx888Planar),
            4 => Ok(PixelFormat::YUV422Packed),
            18 => Ok(PixelFormat::YUV422_UYVYPacked),
            20 => Ok(PixelFormat::YUV422_10Packed),
            21 => Ok(PixelFormat::YUV422_UYVY_10Packed),
            24 => Ok(PixelFormat::YUV444_UYVPacked),
            25 => Ok(PixelFormat::YUV444_UYV_10Packed),
            26 => Ok(PixelFormat::YUV444Packed),
            27 => Ok(PixelFormat::YUV444_10Packed),
            13 => Ok(PixelFormat::YUV422Planar),
            _ => Err(i)
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
enum ListType {
    Undefined = -1,
    Setting = 0,
    Request = 1,
    RequestCtrl = 2,
    Info = 3,
    Statistics = 4,
    SystemSettings = 5,
    IOSubSystem = 6,
    RTCtr = 7,
    CameraDescriptions = 8,
    DeviceSpecificData = 9,
    EventSubSystemSettings = 10,
    EventSubSystemResults = 11,
    ImageMemoryManager = 12
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
enum SearchMode {
    IgnoreLists      = 0x2,
    IgnoreMethods    = 0x4,
    IgnoreProperties = 0x8
}

#[repr(C, packed)]
struct ChannelData {
    pub channel_offset: c_int,
    pub line_pitch: c_int,
    pub pixel_pitch: c_int,
    pub channel_desc: [c_char; 8192],
}

#[repr(C, packed)]
#[derive(Debug)]
struct ImageBuffer {
    pub bytes_per_pixel: c_int,
    pub height: c_int,
    pub width: c_int,
    pub pixel_format: PixelFormat,
    pub size: c_int,
    pub data: *mut c_void,
    pub channel_count: c_int,
    pub channels: *mut ChannelData,
}

struct Image<'a> {
    pub buf: ImageBuffer,
    reqnr: c_int,
    parent: &'a Device,
}

#[link(name = "mvDeviceManager")]
extern "C" {
    // note: DMR_CALL = "" (on Linux)
    // note: MVDMR_API = __attribute__((visibility("default")))
    fn DMR_Init(pDevices: *mut HDMR) -> TDMR_ERROR;
    fn DMR_Close() -> TDMR_ERROR;

    fn DMR_GetDevice(pHDev: *mut HDEV, searchMode: DeviceSearchMode, pSearchString: *const c_char, devNr: c_uint, wildcard: c_char) -> TDMR_ERROR;
    fn DMR_OpenDevice(hDev: HDEV, pHDrv: *mut HDRV) -> TDMR_ERROR;
    fn DMR_CloseDevice(hDrv: HDRV, hDev: HDEV) -> TDMR_ERROR;

    fn DMR_ImageRequestSingle(hDrv: HDRV, requestCtrl: c_int, pRequestUsed: *mut c_int) -> TDMR_ERROR;
    fn DMR_ImageRequestWaitFor(hDrv: HDRV, timeout_ms: c_int, queueNr: c_int, pRequestNr: *mut c_int) -> TDMR_ERROR;
    fn DMR_ImageRequestUnlock(hDrv: HDRV, requestNr: c_int) -> TDMR_ERROR;
    fn DMR_GetImageRequestBuffer(hDrv: HDRV, requestNr: c_int, ppBuffer: *mut *mut ImageBuffer) -> TDMR_ERROR;

    fn DMR_FindList(hDrv: HDRV, pName: *const c_char, typ: ListType, flags: c_uint, pHList: *mut HLIST) -> TDMR_ERROR;

    fn OBJ_GetHandleEx(hList: HLIST, pObjName: *const c_char, phObj: *mut HOBJ, searchMode: c_uint, maxSearchDepth: c_int) -> TPROPHANDLING_ERROR;
    fn OBJ_GetI(hProp: HOBJ, pVal: *mut c_int, index: c_int) -> TPROPHANDLING_ERROR;
    fn OBJ_SetI(hProp: HOBJ, val: c_int, index: c_int) -> TPROPHANDLING_ERROR;
    fn OBJ_GetI64(hProp: HOBJ, pVal: *mut i64, index: c_int) -> TPROPHANDLING_ERROR;
    fn OBJ_SetI64(hProp: HOBJ, val: i64, index: c_int) -> TPROPHANDLING_ERROR;
}

macro_rules! dmr_status2result {
    ($code:expr) => { dmr_status2result!($code, ()) };
    ($code:expr, $ret:expr) => {
        match $code {
            TDMR_ERROR::DMR_NO_ERROR => Ok($ret),
            other => Err(unsafe { mem::transmute(other) }) // TODO make this safe
        }
    }
}

macro_rules! prop_status2result {
    ($code:expr) => { prop_status2result!($code, ()) };
    ($code:expr, $ret:expr) => {
        match $code {
            TPROPHANDLING_ERROR::PROPHANDLING_NO_ERROR => Ok($ret),
            other => Err(unsafe { mem::transmute(other) }) // TODO make this safe
        }
    }
}

macro_rules! c_str {
    ($s:expr) => {
        CString::new($s).unwrap().as_ptr()
    }
}

pub struct Device {
    dmr: HDMR,
    dev: HDEV,
    drv: HDRV,
}

trait ObjProp {
    unsafe fn GetT(hObj: HOBJ, pval: *mut Self, index: c_int) -> TPROPHANDLING_ERROR;
    unsafe fn SetT(hObj: HOBJ, val: Self, index: c_int) -> TPROPHANDLING_ERROR;
}

macro_rules! obj_set_impl {
    ($t:ty, $get:ident, $set:ident) => {
        impl ObjProp for $t {
            unsafe fn GetT(hObj: HOBJ, pval: *mut Self, index: c_int) -> TPROPHANDLING_ERROR { $get(hObj, pval, index) }
            unsafe fn SetT(hObj: HOBJ, val: Self, index: c_int) -> TPROPHANDLING_ERROR { $set(hObj, val, index) }
        }
    }
}

obj_set_impl!(i32, OBJ_GetI, OBJ_SetI);
obj_set_impl!(i64, OBJ_GetI64, OBJ_SetI64);

macro_rules! getter {
    ($name:ident, $prop:expr, $typ:ty) => {
        pub fn $name(&self) -> Result<$typ,TPROPHANDLING_ERROR> {
            self.get_prop::<$typ>("Base", $prop, 0)
        }
    };
    ($name:ident, $prop:expr, $typ:ty, $conv_var:ident: $conv_typ:ty => $conv_body:expr) => {
        pub fn $name(&self) -> Result<$typ,TPROPHANDLING_ERROR> {
            match self.get_prop::<$conv_typ>("Base", $prop, 0) {
                Ok($conv_var) => $conv_body,
                Err(e) => Err(e)
            }
        }
    }
}
macro_rules! setter {
    ($name:ident, $prop:expr, $typ:ty) => {
        pub fn $name(&self, val: $typ) -> Result<(),TPROPHANDLING_ERROR> {
            self.set_prop::<$typ>("Base", $prop, val, 0)
        }
    };
    ($name:ident, $prop:expr, $typ:ty, $conv_var:ident: $conv_typ:ty => $conv_body:expr) => {
        pub fn $name(&self, $conv_var: $conv_typ) -> Result<(),TPROPHANDLING_ERROR> {
            self.set_prop::<$typ>("Base", $prop, $conv_body, 0)
        }
    }
}

impl Device {
    pub fn new() -> Result<Device,TDMR_ERROR> {
        let mut this = Device { dmr: HDMR(0), dev: HDEV(0), drv: HDRV(0) };
        try!(dmr_status2result!(unsafe { DMR_Init(&mut this.dmr) }));

        try!(dmr_status2result!(unsafe { DMR_GetDevice(&mut this.dev, DeviceSearchMode::Serial, c_str!("*"), 0, b'*' as c_char) }));
        dmr_status2result!(unsafe { DMR_OpenDevice(this.dev, &mut this.drv) }, this)
    }

    getter!(get_height, "Height", i64);
    setter!(set_height, "Height", i64);
    getter!(get_width, "Width", i64);
    setter!(set_width, "Width", i64);
    getter!(get_reverse_x, "ReverseX", bool, i: i32 => Ok(i == 1));
    setter!(set_reverse_x, "ReverseX", i32, b: bool => if b { 1 } else { 0 });
    getter!(get_reverse_y, "ReverseY", bool, i: i32 => Ok(i == 1));
    setter!(set_reverse_y, "ReverseY", i32, b: bool => if b { 1 } else { 0 });
    getter!(get_pixel_format, "PixelFormat", PixelFormat, i: i32 => match PixelFormat::from_prop(i) {
                                                                        Ok(px) => Ok(px),
                                                                        Err(i) => panic!("Could not convert {} to PixelFormat", i)});
    setter!(set_pixel_format, "PixelFormat", i32, px: PixelFormat => match PixelFormat::to_prop(px) {
                                                                        Ok(i) => i,
                                                                        Err(px) => panic!("Could not convert {:?} to int", px)});

    fn set_prop<T: ObjProp>(&self, setting: &str, prop: &str, value: T, index: c_int) -> Result<(),TPROPHANDLING_ERROR> {
        prop_status2result!(unsafe { T::SetT(try!(self.get_setting_prop(setting, prop)), value, index) })
    }

    fn get_prop<T: ObjProp>(&self, setting: &str, prop: &str, index: c_int) -> Result<T,TPROPHANDLING_ERROR> {
        let mut value: T = unsafe { mem::uninitialized() };
        prop_status2result!(unsafe { T::GetT(try!(self.get_setting_prop(setting, prop)), &mut value, index) }, value)
    }

    fn get_setting_prop(&self, setting: &str, prop: &str) -> Result<HOBJ,TPROPHANDLING_ERROR> {
        self.get_driver_property(prop, setting, ListType::Setting)
    }

    fn get_driver_property(&self, prop: &str, list: &str, typ: ListType) -> Result<HOBJ,TPROPHANDLING_ERROR> {
        self.get_driver_feature(prop, "property", list, typ, SearchMode::IgnoreLists as u32 | SearchMode::IgnoreMethods as u32)
    }

    fn get_driver_feature(&self, feature_name: &str, feature_type: &str, list: &str, list_type: ListType, mode: u32) -> Result<HOBJ,TPROPHANDLING_ERROR> {
        let mut base: HLIST = unsafe { mem::uninitialized() };
        let mut obj: HOBJ = unsafe { mem::uninitialized() };
        try!(dmr_status2result!(unsafe { DMR_FindList(self.drv, c_str!(list), list_type, 0, &mut base) }));
        prop_status2result!(unsafe { OBJ_GetHandleEx(base, c_str!(feature_name), &mut obj, mode, c_int::max_value()) }, obj)
    }

    pub fn request(&self) -> Result<Image,TDMR_ERROR> {
        try!(dmr_status2result!(unsafe { DMR_ImageRequestSingle(self.drv, 0, ptr::null_mut()) }));
        let mut reqnr: c_int = 0;
        try!(dmr_status2result!(unsafe { DMR_ImageRequestWaitFor(self.drv, -1, 0, &mut reqnr) }));
        let mut image_buf = ImageBuffer { bytes_per_pixel: 0, channel_count: 0, height: 0, size: 0, width: 0, channels: ptr::null_mut(), pixel_format: PixelFormat::Mono8, data: ptr::null_mut() };
        dmr_status2result!(unsafe { DMR_GetImageRequestBuffer(self.drv, reqnr, &mut &mut image_buf as *mut &mut ImageBuffer as *mut *mut ImageBuffer) }, Image { buf: image_buf, reqnr: reqnr, parent: self })
    }

    pub fn close(&self) -> Result<(),TDMR_ERROR> {
        try!(dmr_status2result!(unsafe { DMR_CloseDevice(self.drv, self.dev) }));
        dmr_status2result!(unsafe { DMR_Close() })
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
        &unsafe { slice::from_raw_parts(mem::transmute(self.buf.data), self.buf.size as usize) }
    }
}

impl<'a> Drop for Image<'a> {
    fn drop(&mut self) {
        unsafe { DMR_ImageRequestUnlock(self.parent.drv, self.reqnr); }
    }
}

