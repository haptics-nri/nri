extern crate libc;
use self::libc::{c_void, c_int, c_char, c_uchar, c_float, c_double};
use std::default::Default;
use std::f32;
use std::ffi::CString;
use std::fmt;
use std::ops::Deref;

// FIXME this is totally undefined behavior all the time!
// for literals: b"str\0".as_ptr()
// for strings: keep the CString alive until the C function returns
macro_rules! c_str {
    ($s:expr) => {
        CString::new($s).unwrap().as_ptr()
    }
}

macro_rules! try_opt {
    ($e:expr) => {
        match $e {
            Option::Some(v) => v,
            Option::None    => return Option::None
        }
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
    fn lofa_sensor_get(that: Handle) -> c_uchar;
    fn lofa_sensor_set(that: Handle, byte: c_uchar);
}

#[repr(C)]
pub struct Version {
    major:    c_int,
    minor:    c_int,
    revision: c_int
}

pub struct Double(c_double);

impl fmt::Debug for Double {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(),fmt::Error> {
        write!(f, "{: >7.3}", self.deref())
    }
}

impl Deref for Double {
    type Target = f64;

    fn deref(&self) -> &f64 {
        &self.0
    }
}

#[derive(Debug)]
pub struct XYZ {
    x: Double,
    y: Double,
    z: Double
}

type Handle = *mut c_void;

pub struct Device {
    pdev: Handle
}

impl Device {
    pub fn new(opt: DeviceOptions) -> Device {
        Device { pdev: unsafe { lofa_create_sensor(opt.buffer as c_int, opt.factor as c_float) } }
    }

    pub fn connect(&self, opt: ConnectOptions) -> Result<(),()> {
        match unsafe { lofa_sensor_connect(self.pdev, c_str!(opt.path), opt.baud) } {
            true => Ok(()),
            false => Err(())
        }
    }

    pub fn disconnect(&self, block: bool) {
        unsafe { lofa_sensor_disconnect(self.pdev, block); }
    }

    pub fn set(&self, conf: Settings) {
        unsafe { lofa_sensor_set(self.pdev, conf.encode()) };
    }

    pub fn get(&self) -> Option<Settings> {
        Settings::decode(unsafe { lofa_sensor_get(self.pdev) })
    }

    pub fn read(&self) -> XYZ {
        unsafe { lofa_sensor_read(self.pdev) }
    }
}

pub struct DeviceOptions {
    pub buffer: isize,
    pub factor: f32
}

pub struct ConnectOptions<'a> {
    pub path: &'a str,
    pub baud: i32
}

pub mod settings {
    macro_rules! make_enum {
        ($s:ident: $t:ty => $d:ty { $($body:tt,)* }) => {
            make_enum!(PARSE $s: $t => $d, [], [], { $($body,)* });
        };

        (PARSE $s:ident: $t:ty => $d:ty, [$($hz:tt),*], [$($nohz:tt),*],
               { ($hzv:ident, $hzh:expr, $hzd:expr), $($rest:tt,)* }) => {
            make_enum!(PARSE $s: $t => $d, [$($hz,)* ($hzv, $hzh, $hzd)], [$($nohz),*], { $($rest,)* });
        };
        (PARSE $s:ident: $t:ty => $d:ty, [$($hz:tt),*], [$($nohz:tt),*],
               { ($nzv:ident, $nzd:expr), $($rest:tt,)* }) => {
            make_enum!(PARSE $s: $t => $d, [$($hz),*], [$($nohz,)* ($nzv, $nzd)], { $($rest,)* });
        };
        (PARSE $s:ident: $t:ty => $d:ty, [$($hz:tt),*], [$($nohz:tt),*],
               { }) => {
            make_enum!(OUT $s: $t => $d, [$($hz),*], [$($nohz),*]);
        };

        (OUT $s:ident: $t:ty => $d:ty, [$(($hz_variant:ident, $hz_hz:expr, $hz_dev:expr)),*],
                                       [$(($nohz_variant:ident, $nohz_dev:expr)),*]) => {
            #[derive(Debug, Copy, Clone)]
            pub enum $s {
                $($hz_variant,)*
                $($nohz_variant),*
            }

            impl $s {
                pub fn from_device(i: $d) -> Option<$s> {
                    match i {
                        $($hz_dev   => Some($s::$hz_variant),)*
                        $($nohz_dev => Some($s::$nohz_variant),)*
                        _           => None
                    }
                }

                pub fn to_device(self) -> $d {
                    match self {
                        $($s::$hz_variant   => $hz_dev,)*
                        $($s::$nohz_variant => $nohz_dev),*
                    }
                }

                pub fn from_hz(i: $t) -> Option<$s> {
                    match i {
                        $($hz_hz => Some($s::$hz_variant),)*
                        _        => None
                    }
                }

                pub fn to_hz(self) -> Option<$t> {
                    match self {
                        $($s::$hz_variant   => Some($hz_hz),)*
                        $($s::$nohz_variant => None),*
                    }
                }
            }
        }
    }

    make_enum! {
        Speed: u8 => u8 {
            (Hz30   , 30   , 3),
            (Hz100  , 100  , 2),
            (Hz333  , 333  , 1),
            (Hz1000 , 1000 , 0),
        }
    }

    make_enum! {
        Filter: u8 => u8 {
            (None  ,       0),
            (Hz15  , 15  , 3),
            (Hz50  , 50  , 2),
            (Hz150 , 150 , 1),
        }
    }

    make_enum! {
        Mode: &'static str => u8 {
            (Raw   , "raw"   , 0),
            (Force , "force" , 1),
        }
    }

    make_enum! {
        State: () => u8 {
            (NoSensor         , 0),
            (OverloadX        , 1),
            (OverloadY        , 2),
            (OverloadZ        , 3),
            (SensorFailure    , 4),
            (SensorOk         , 5),
            (ConnectionFailure, 6),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Settings {
    pub speed:  settings::Speed,
    pub filter: settings::Filter,
    pub mode:   settings::Mode,
    pub state:  settings::State
}

impl Settings {
    pub fn new() -> Settings {
        Default::default()
    }

    pub fn set_speed(mut self, speed: settings::Speed) -> Settings { self.speed = speed; self }
    pub fn set_filter(mut self, filter: settings::Filter) -> Settings { self.filter = filter; self }
    pub fn set_mode(mut self, mode: settings::Mode) -> Settings { self.mode = mode; self }
    pub fn set_state(mut self, state: settings::State) -> Settings { self.state = state; self }

    fn encode(self) -> u8 {
        (self.mode.to_device())
            | (self.filter.to_device() << 1)
            | (self.speed.to_device()  << 3)
            | (self.state.to_device()  << 5)
    }

    fn decode(byte: u8) -> Option<Settings> {
        Some(Settings {
            mode:   try_opt!(settings::Mode::  from_device( byte       & 0b0000_0001)),
            filter: try_opt!(settings::Filter::from_device((byte >> 1) & 0b0000_0011)),
            speed:  try_opt!(settings::Speed:: from_device((byte >> 3) & 0b0000_0011)),
            state:  try_opt!(settings::State:: from_device((byte >> 5) & 0b0000_0111)),
        })
    }
}

impl Default for DeviceOptions {
    fn default() -> DeviceOptions {
        DeviceOptions { buffer: -1, factor: f32::NAN }
    }
}

impl Default for ConnectOptions<'static> {
    fn default() -> ConnectOptions<'static> {
        ConnectOptions { path: "/dev/ttyACM0", baud: -1 }
    }
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            state:  settings::State:: SensorOk,
            speed:  settings::Speed:: Hz100   ,
            filter: settings::Filter::Hz15    ,
            mode:   settings::Mode::  Force
        }
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

