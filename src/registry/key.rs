
use std::{
    ptr::null_mut,
    mem::MaybeUninit,
    ffi::OsStr,
};

use winapi::{
    shared::{
        minwindef::HKEY,
    },
    um::{
        processthreadsapi::GetCurrentProcess,
        handleapi::DuplicateHandle,
        winnt::{
            HANDLE,
            KEY_READ,
            KEY_WRITE,
            DUPLICATE_SAME_ACCESS,
        },
        winreg::{
            self,
            RegOpenKeyExW,
            RegCreateKeyExW,
            RegCloseKey,
        }
    },
};

use super::{
    KeyExt, Name, Result, Error, to_wstr
};

pub struct Key(pub(super) HKEY);

pub const HKEY_CURRENT_USER: Key = Key(winreg::HKEY_CURRENT_USER);
pub const HKEY_LOCAL_MACHINE: Key = Key(winreg::HKEY_LOCAL_MACHINE);

impl KeyExt for Key {
    unsafe fn handle(&self) -> HKEY { self.0 }
}

impl Key {

    /// open a key for writing
    pub fn open(&self, path: impl AsRef<Name>) -> Result<Key> {
        tracing::trace!("opening: {:?}", path.as_ref());
        unsafe {
            let path = to_wstr(path);
            let mut subkey = MaybeUninit::<HKEY>::uninit();

            Error::check_code(
                RegOpenKeyExW(
                    /* hKey       */ self.0,
                    /* lpSubKey   */ path.as_ptr(),
                    /* ulOptions  */ 0u32,
                    /* samDesired */ KEY_READ,
                    /* phkResult  */ subkey.as_mut_ptr()
                )
            )?;

            Ok(Self(subkey.assume_init()))
        }
    }

    /// create a new or open an existing key for writing
    pub fn create(&self, path: impl AsRef<OsStr>) -> Result<Key> {
        tracing::trace!("creating: {:?}", path.as_ref());
        unsafe {
            let path = to_wstr(path);
            let mut subkey = MaybeUninit::<HKEY>::uninit();

            Error::check_code(
                RegCreateKeyExW(
                    /* hKey                 */ self.0,
                    /* lpSubKey             */ path.as_ptr(),
                    /* Reserved             */ 0u32,
                    /* lpClass              */ null_mut(),
                    /* dwOptions            */ 0u32,
                    /* samDesired           */ KEY_WRITE,
                    /* lpSecurityAttributes */ null_mut(),
                    /* phkResult            */ subkey.as_mut_ptr(),
                    /* lpdwDisposition      */ null_mut(),
                )
            )?;

            Ok(Self(subkey.assume_init()))
        }
    }

    pub unsafe fn clone_handle(src_handle: HKEY) -> Result<Key> {

        let mut dst_handle = MaybeUninit::<HANDLE>::uninit();

        Error::check_true(DuplicateHandle(
            /* hSourceProcessHandle */ GetCurrentProcess(),
            /* hSourceHandle        */ src_handle as _,
            /* hTargetProcessHandle */ GetCurrentProcess(),
            /* lpTargetHandle       */ dst_handle.as_mut_ptr(),
            /* dwDesiredAccess      */ 0,
            /* bInheritHandle       */ 0,
            /* dwOptions            */ DUPLICATE_SAME_ACCESS,
        ))?;

        Ok(Key(dst_handle.assume_init() as _))
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        unsafe { RegCloseKey(self.0); }
    }
}
