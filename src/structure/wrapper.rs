extern crate libc;

#[repr(C)]
pub enum OniStatus {
	Ok             = 0,
	Error          = 1,
	NotImplemented = 2,
	NotSupported   = 3,
	BadParameter   = 4,
	OutOfFlow      = 5,
	NoDevice       = 6,
	TimeOut        = 102,
}

#[link(name = "OpenNI2")]
extern "C"
{
    pub fn oniInitialize(apiVersion: i32) -> OniStatus;
    pub fn oniShutdown();
}

