extern crate libc;
use self::libc::{c_void, c_int, c_uint, c_char};
use std::mem;
use std::ffi::CString;

#[derive(Clone, Copy)] pub struct HDMR(c_int);
#[derive(Clone, Copy)] pub struct HDEV(c_int);
#[derive(Clone, Copy)] pub struct HDRV(c_int);

#[repr(C)]
#[derive(Debug)]
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
enum DeviceSearchMode {
    Serial     = 1,
    Family     = 2,
    Product    = 3,
    UseDevID   = 0x8000
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
}

macro_rules! status2result {
    ($code:expr) => { status2result!($code, ()) };
    ($code:expr, $ret:expr) => {
        match $code {
            TDMR_ERROR::DMR_NO_ERROR => Ok($ret),
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

impl Device {
    pub fn new() -> Result<Device,TDMR_ERROR> {
        let mut this = Device { dmr: HDMR(0), dev: HDEV(0), drv: HDRV(0) };
        try!(status2result!(unsafe { DMR_Init(&mut this.dmr) }));

        try!(status2result!(unsafe { DMR_GetDevice(&mut this.dev, DeviceSearchMode::Serial, c_str!("*"), 0, b'*' as c_char) }));
        status2result!(unsafe { DMR_OpenDevice(this.dev, &mut this.drv) }, this)
    }

    pub fn close(&self) -> Result<(),TDMR_ERROR> {
        try!(status2result!(unsafe { DMR_CloseDevice(self.drv, self.dev) }));
        status2result!(unsafe { DMR_Close() })
    }
}

