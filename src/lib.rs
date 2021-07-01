//! Windows Service Wrapper
//!
//! This crate provides a simple mechanism to host a rust application as a
//! windows services. It is meant to be a batteries included experience. To that
//! end it provides several features. There is a built it command line interface,
//! with easy configuration management via `structop` and 'serde'.
//!
//! ```
//! use tokio::select;
//! use winsvc::QuitSignal;
//! use structopt::StructOpt;
//! use serde::{Serialize,Deserialize};
//!
//! #[derive(StructOp,Serialize,Deserialize,Debug)]
//! struct Configuration {
//!     option_1: String,
//!     option_2: isize,
//! }
//!
//! #[winsvc::main]
//! async fn main(config: Config, quit_signal: QuitSignal) {
//!     select!{ quit_signal };
//! }
//!
//! ```


pub mod service_dispatcher;
pub mod service_control_manager;

#[cfg(feature = "async_main")]
pub mod async_service_main;

#[cfg(feature = "serde_config")]
pub mod serde_config;

#[cfg(feature = "std_cli")]
pub mod std_cli;

use std::{
    fmt,
    ptr::null_mut,
    ffi::{OsStr,OsString},
    path::PathBuf,
    mem::MaybeUninit,
    os::windows::ffi::{OsStrExt,OsStringExt},
};

use winapi::{
    um::{
        winnt::WCHAR,
        psapi::GetModuleFileNameExW,
        processthreadsapi::GetCurrentProcess,
        errhandlingapi::GetLastError,
    },
    shared::minwindef::MAX_PATH,
};


#[derive(Copy,Clone,Eq,PartialEq)]
pub struct Error{pub code:u32}

pub type Result<T> = std::result::Result<T,Error>;

/// types that can be used for true/false checks
pub trait IsTrue {
    fn is_true(&self) -> bool;
}

impl IsTrue for bool { fn is_true(&self) -> bool { *self } }
impl IsTrue for u32 { fn is_true(&self) -> bool { *self != 0 } }
impl IsTrue for i32 { fn is_true(&self) -> bool { *self != 0 } }

impl Error {

    pub fn from_last() -> Self {
        Self{code:unsafe{GetLastError()}}
    }

    pub fn check_true(value: impl IsTrue) -> Result<()> {
        if value.is_true() {
            Ok(())
        } else {
            Err(Error::from_last())
        }
    }

    pub fn check_code(code: i32) -> Result<()> {
        if code == 0 {
            Ok(())
        } else {
            Err(Error{code:code as u32})
        }
    }

    fn format (&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match crate::format_error (self.code) {
            Some(message) => f.write_str(&message),
            None => write!(f, "unknown error code {}", self.code)
        }
    }
}

impl std::error::Error for Error {}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}] ", self.code)?;
        self.format(f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { self.format(f) }
}





fn osstr_to_wchars (os_str: &OsStr) -> Vec<u16> {
    use std::iter::once;
    os_str.encode_wide ().chain (once (0)).collect ()
}

fn to_wstr(input: impl AsRef<OsStr>) -> Vec<u16>
{
    osstr_to_wchars(input.as_ref())
}

pub mod registry;
pub mod service_configuration {
    use super::{ Result, registry::{KeyExt, HKEY_LOCAL_MACHINE} };

    fn get_service_key_path(name: &str) -> String {
        format!("SYSTEM\\CurrentControlSet\\Services\\{}", name)
    }

    #[cfg(feature = "serde_config")]
    pub fn save<C>(name: &str, value: &C) -> Result<()> where C: serde::ser::Serialize {
        HKEY_LOCAL_MACHINE.create(get_service_key_path(name))?.set_object("Configuration", value)
    }

    #[cfg(feature = "serde_config")]
    pub fn load<C>(name: &str) -> Result<C> where C: serde::de::DeserializeOwned {
        HKEY_LOCAL_MACHINE.open(get_service_key_path(name))?.query_object("Configuration")
    }
}




pub fn get_this_module_filename_raw() -> Result<Vec<u16>> {
    let mut buffer = Vec::<u16>::new();

    buffer.resize(MAX_PATH, 0);

    let len = unsafe { GetModuleFileNameExW(GetCurrentProcess(), null_mut(), buffer.as_mut_ptr(), buffer.len() as u32) };

    if len > 0 {
        buffer.resize(len as usize, 0);
        Ok(buffer)
    } else {
        Err(Error::from_last())
    }
}

pub fn get_this_module_filename() -> Result<PathBuf> {
    get_this_module_filename_raw().map(|buffer|PathBuf::from(OsString::from_wide(&buffer)))
}




/// Format a Win32 error code into a descriptive message.
pub fn format_error(code: u32) -> Option<String> {

    use winapi::um::{
        winbase::{
        FormatMessageW,
        FORMAT_MESSAGE_FROM_SYSTEM
        }
    };

    unsafe {

        const BUF_SIZE: usize = 1024;

        let mut buffer: [MaybeUninit<WCHAR>; BUF_SIZE] = MaybeUninit::uninit ().assume_init();

        let len = FormatMessageW(
            FORMAT_MESSAGE_FROM_SYSTEM,
            null_mut(),
            code, 0,
            buffer.as_mut_ptr() as *mut u16,
            BUF_SIZE as u32,
            null_mut ()
        ) as usize;

        if len > 0 {

            let char_buffer : &[WCHAR; BUF_SIZE] = std::mem::transmute(&buffer);

            let slice = &char_buffer[..len];

            let message = std::ffi::OsString::from_wide (slice);

            let message = String::from (message.to_string_lossy ());

            let message = String::from (message.trim ());

            Some(message)

        } else {
            None
        }
    }

}
