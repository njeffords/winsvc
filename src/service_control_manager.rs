
use std::{
    ptr::null_mut,
    mem::MaybeUninit,
};

use winapi::{
    shared::winerror::{
        ERROR_GEN_FAILURE,
    },
    um::{
        synchapi::SleepEx,
        winsvc::{
            OpenServiceW,
            StartServiceW,
            DeleteService,
            ControlService,
            CreateServiceW,
            OpenSCManagerW,
            CloseServiceHandle,
            QueryServiceStatusEx,
            SC_HANDLE,
            SC_MANAGER_ALL_ACCESS,
            SERVICE_ALL_ACCESS,
            SERVICE_STATUS,
            SERVICE_STATUS_PROCESS,
            SC_STATUS_PROCESS_INFO,
            SERVICE_RUNNING,
            SERVICE_START_PENDING,
            SERVICE_STOPPED,
            SERVICE_STOP_PENDING,
            SERVICE_CONTROL_STOP,
        },
        winnt::{
            SERVICE_WIN32_OWN_PROCESS,
            SERVICE_AUTO_START,
            SERVICE_ERROR_NORMAL,
        },
    },
};

use super::{
    to_wstr,
    Error, Result,
    get_this_module_filename_raw,
};

#[derive(Copy,Clone)]
pub enum Access {
    All,
}

impl Access {
    fn into_raw(self) -> u32 {
        match self {
            Access::All => SC_MANAGER_ALL_ACCESS
        }
    }
}

pub const LOCAL_SERVICE: &'static str = "NT AUTHORITY\\LocalService";
pub const NETWORK_SERVICE: &'static str = "NT AUTHORITY\\NetworkService";

pub struct Service(SC_HANDLE);

pub struct ServiceControlManager(SC_HANDLE);

trait StatusStruct{
    const INFO_LEVEL: u32;
}

impl StatusStruct for SERVICE_STATUS_PROCESS {
    const INFO_LEVEL: u32 = SC_STATUS_PROCESS_INFO;
}

impl Service {
    pub fn delete(&self) -> Result<()> {
        if unsafe { DeleteService(self.0) } != 0 {
            Ok(())
        } else {
            Err(Error::from_last())
        }
    }

    pub fn start(&self) -> Result<()> {
        if !self.wait_for_start()? {
            self.raw_service_start()?;
            if !self.wait_for_start()? {
                Err(Error{code:ERROR_GEN_FAILURE})
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    pub fn stop(&self) -> Result<()> {
        if !self.wait_for_stop()? {
            self.raw_control_service(SERVICE_CONTROL_STOP)?;
            if !self.wait_for_stop()? {
                Err(Error{code:ERROR_GEN_FAILURE})
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    fn raw_service_start(&self) -> Result<()> {
        unsafe {
            let res = StartServiceW(
                self.0,
                0,
                null_mut()
            );

            if res != 0 {
                Ok(())
            } else {
                Err(Error::from_last())
            }
        }
    }

    fn raw_control_service(&self, control: u32) -> Result<SERVICE_STATUS> {
        unsafe {

            let mut status = MaybeUninit::<SERVICE_STATUS>::uninit();

            let res = ControlService(
                self.0,
                control,
                status.as_mut_ptr(),
            );

            if res != 0 {
                Ok(status.assume_init())
            } else {
                Err(Error::from_last())
            }
        }
    }

    fn raw_query_service_status<T: StatusStruct>(&self) -> Result<T> {
        unsafe {

            let mut status = MaybeUninit::<T>::uninit();
            let mut bytes_needed : u32 = 0;

            let res = QueryServiceStatusEx(self.0,
                T::INFO_LEVEL,
                status.as_mut_ptr() as * mut _,
                std::mem::size_of::<T>() as u32,
                &mut bytes_needed as *mut _,
            );

            if res != 0 {
                Ok(status.assume_init())
            } else {
                Err(Error::from_last())
            }
        }
    }

    fn wait_for_start(&self)-> Result<bool> {

        let sleep = |timeout|unsafe {SleepEx(timeout, 1)};

        let query = || -> Result<(u32,u32)> {
            let st = self.raw_query_service_status::<SERVICE_STATUS_PROCESS>()?;
            Ok((st.dwCurrentState, st.dwWaitHint))
        };

        let (state, wait) = query()?;

        match state {
            SERVICE_RUNNING => Ok(true),
            SERVICE_START_PENDING => {
                sleep(wait);
                loop {
                    let (state,wait) = query()?;
                    match state {
                        SERVICE_START_PENDING => {sleep(wait);continue},
                        SERVICE_RUNNING => break Ok(true),
                        _ => return Err(Error{code:ERROR_GEN_FAILURE}),
                    }
                }
            },
            _ => Ok(false),
        }
    }

    fn wait_for_stop(&self)-> Result<bool> {

        let sleep = |timeout|unsafe {SleepEx(timeout, 1)};

        let query = || -> Result<(u32,u32)> {
            let st = self.raw_query_service_status::<SERVICE_STATUS_PROCESS>()?;
            Ok((st.dwCurrentState, st.dwWaitHint))
        };

        let (state, wait) = query()?;

        match state {
            SERVICE_STOPPED => Ok(true),
            SERVICE_STOP_PENDING => {
                sleep(wait);
                loop {
                    let (state,wait) = query()?;
                    match state {
                        SERVICE_STOP_PENDING => {sleep(wait);continue},
                        SERVICE_STOPPED => break Ok(true),
                        _ => return Err(Error{code:ERROR_GEN_FAILURE}),
                    }
                }
            },
            _ => Ok(false),
        }
    }
}

impl ServiceControlManager {

    pub fn open_local(access: Access) -> Result<Self> {
        let handle = unsafe { OpenSCManagerW(null_mut(), null_mut(), access.into_raw()) };
        if handle != null_mut () {
            Ok(Self(handle))
        } else {
            Err(Error::from_last())
        }
    }

    pub fn open_service(
        &mut self,
        service_name: &str,
    ) -> Result<Service> {

        let service_name = to_wstr(service_name);

        let handle = unsafe { OpenServiceW(self.0, service_name.as_ptr(), SC_MANAGER_ALL_ACCESS) };

        if handle != null_mut() {
            Ok(Service(handle))
        } else {
            Err(Error::from_last())
        }
    }

    /// create a service that starts this executable with the specified arguments
    pub fn create_self_service_simple(
        &mut self,
        service_name: &str,
        display_name: &str,
        arguments: &[&str],
        service_start_name: &str,
    ) -> Result<Service> {

        let mut binary_path_name = get_this_module_filename_raw()?;
        let raw_service_name = to_wstr(service_name);
        let display_name = to_wstr(display_name);
        let service_start_name = to_wstr(service_start_name);

        for argument in arguments {
            tracing::trace!("arg: {}", argument);
            assert!(!argument.contains('"'));
            binary_path_name.push(' ' as u16);
            let mut argument = to_wstr(&argument);
            argument.truncate(argument.len() - 1);
            if argument.contains(&(' ' as u16)) {
                argument.insert(0, '"' as u16);
                argument.push('"' as u16);
            }
            binary_path_name.append(&mut argument);
        }
        binary_path_name.push(0);

        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        tracing::trace!("binary_path_name: {:?}", OsString::from_wide(&binary_path_name));

        let password: [u16;1] = [0];

        let handle = unsafe {
            CreateServiceW(
                self.0,
                raw_service_name.as_ptr(),
                display_name.as_ptr(),
                SERVICE_ALL_ACCESS,
                SERVICE_WIN32_OWN_PROCESS,
                SERVICE_AUTO_START,
                SERVICE_ERROR_NORMAL,
                binary_path_name.as_ptr(),
                null_mut(), // lpLoadOrderGroup
                null_mut(), // lpdwTagId
                null_mut(), // lpDependencies
                service_start_name.as_ptr(),
                password.as_ptr(),
            )
        };

        if handle == null_mut() {
            return Err(Error::from_last());
        }

        Ok(Service(handle))
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        unsafe { CloseServiceHandle (self.0) };
    }
}

impl Drop for ServiceControlManager {
    fn drop(&mut self) {
        unsafe { CloseServiceHandle (self.0) };
    }
}