
use std::{
    ptr::null_mut,
    sync::mpsc,
    ffi::c_void,
};

use winapi::{
    shared::winerror::{
        NO_ERROR,
        ERROR_CALL_NOT_IMPLEMENTED,
    },
    um::{
        winsvc::{
            SERVICE_RUNNING,
            SERVICE_START_PENDING,
            SERVICE_PAUSE_PENDING,
            SERVICE_STOP_PENDING,
            SERVICE_PAUSED,
            SERVICE_STOPPED,

            SERVICE_CONTROL_INTERROGATE,
            SERVICE_CONTROL_PARAMCHANGE,
            SERVICE_CONTROL_PAUSE,
            SERVICE_CONTROL_CONTINUE,
            SERVICE_CONTROL_STOP,

            SERVICE_STATUS,
            SERVICE_TABLE_ENTRYW,
            SERVICE_STATUS_HANDLE,

            SetServiceStatus,
            StartServiceCtrlDispatcherW,
            RegisterServiceCtrlHandlerExW,
        },
        winnt::{
            SERVICE_WIN32_OWN_PROCESS,
        },
    },
};

use super::{Result, Error, to_wstr};

pub type RawServiceMain = unsafe extern "system" fn(u32, *mut *mut u16);

pub struct ServiceArgs{
    _argc: u32, _argv: *mut *mut u16
}

impl ServiceArgs {
    pub unsafe fn from_raw(_argc: u32, _argv: *mut *mut u16) -> Self {
        Self{ _argc, _argv }
    }
}

pub struct ServiceStatus(SERVICE_STATUS_HANDLE,SERVICE_STATUS);

#[derive(Clone,Debug)]
pub enum ServiceEvent{
    Interrogate,
    ParamChange,
    Pause,
    Continue,
    Stop,
}

pub const SERVICE_EVENT_NOT_IMPLEMENTED : Error = Error{code: ERROR_CALL_NOT_IMPLEMENTED};

pub type ServiceEventResult = Result<()>;

pub type ServiceEvents = mpsc::Receiver<ServiceEvent>;

impl ServiceEvent {
    unsafe fn from_raw(control: u32, _event: u32, _data: *mut c_void) -> Option<Self> {
        match control {
            SERVICE_CONTROL_INTERROGATE => Some(Self::Interrogate),
            SERVICE_CONTROL_PARAMCHANGE => Some(Self::ParamChange),
            SERVICE_CONTROL_PAUSE => Some(Self::Pause),
            SERVICE_CONTROL_CONTINUE => Some(Self::Continue),
            SERVICE_CONTROL_STOP => Some(Self::Stop),
            _ => None
        }
    }
}

impl ServiceStatus {

    pub fn send(&mut self) -> Result<()> {
        tracing::trace!("sending service status: {}", self.1.dwCurrentState);
        let res = unsafe { SetServiceStatus(self.0, &mut self.1) };
        if res != 0 {
            Ok(())
        } else {
            Err(Error::from_last())
        }
    }

    pub fn pausing(&mut self, wait_hint: std::time::Duration) ->  Result<()> {
        self.1.dwCurrentState = SERVICE_PAUSE_PENDING;
        self.1.dwControlsAccepted = 0;
        self.1.dwCheckPoint += 1;
        self.1.dwWaitHint = wait_hint.as_millis() as u32;
        self.send()
    }

    pub fn paused(&mut self) ->  Result<()> {
        self.1.dwCurrentState = SERVICE_PAUSED;
        self.1.dwControlsAccepted = SERVICE_CONTROL_STOP|SERVICE_CONTROL_CONTINUE;
        self.1.dwCheckPoint += 1;
        self.send()
    }

    pub fn starting(&mut self, wait_hint: std::time::Duration) ->  Result<()> {
        self.1.dwCurrentState = SERVICE_START_PENDING;
        self.1.dwControlsAccepted = 0;
        self.1.dwCheckPoint += 1;
        self.1.dwWaitHint = wait_hint.as_millis() as u32;
        self.send()
    }

    pub fn running(&mut self) -> Result<()> {
        self.1.dwCurrentState = SERVICE_RUNNING;
        self.1.dwControlsAccepted = SERVICE_CONTROL_STOP|SERVICE_CONTROL_PAUSE;
        self.send()
    }

    pub fn stopping(&mut self, wait_hint: std::time::Duration) ->  Result<()> {
        self.1.dwCurrentState = SERVICE_STOP_PENDING;
        self.1.dwControlsAccepted = 0;
        self.1.dwCheckPoint += 1;
        self.1.dwWaitHint = wait_hint.as_millis() as u32;
        self.send()
    }

    pub fn stopped(&mut self) -> Result<()> {
        self.1.dwCurrentState = SERVICE_STOPPED;
        self.1.dwControlsAccepted = 0;
        self.send()
    }

}

unsafe extern "system" fn handler_function_ex<T>(control: u32, event: u32, data: *mut c_void, context: *mut c_void) -> u32
    where T: FnMut(ServiceEvent)->Result<()>
{
    tracing::trace!("service control handler received: {}, {}", control, event);
    let code = if let Some(event) = ServiceEvent::from_raw(control, event, data) {
        let result = ({&mut*(context as *mut T)})(event);
        match result {
            Ok(()) => NO_ERROR,
            Err(Error{code}) => code,
        }
    } else {
        ERROR_CALL_NOT_IMPLEMENTED
    };
    if control != SERVICE_CONTROL_INTERROGATE {
        code
    } else {
        NO_ERROR
    }
}

pub fn register_service_ctrl_handler<T>(service_name: &str, handler: T) -> Result<ServiceStatus> where T: FnMut(ServiceEvent)->Result<()> {

    let service_name = to_wstr(service_name);
    let context = Box::into_raw(Box::new(handler));

    let handle = unsafe { RegisterServiceCtrlHandlerExW(
        service_name.as_ptr(),
        Some(handler_function_ex::<T>),
        context as *mut _,
    ) };

    if handle != null_mut() {

        let status = SERVICE_STATUS{
            dwServiceType: SERVICE_WIN32_OWN_PROCESS,
            dwCurrentState: SERVICE_START_PENDING,
            dwControlsAccepted: 0,
            dwWin32ExitCode: 0,
            dwServiceSpecificExitCode: 0,
            dwCheckPoint: 0,
            dwWaitHint: 1000,
        };

        Ok(ServiceStatus(handle,status))
    } else {
        drop(unsafe{ Box::from_raw(context) });
        Err(Error::from_last())
    }
}


pub unsafe fn raw_service_main_wrapper(
    argc: u32,
    argv: *mut *mut u16,
    function: fn(ServiceArgs)
) {
    function(ServiceArgs::from_raw(argc,argv))
}

#[macro_export]
macro_rules! start_service_ctrl_dispatcher {
    ( $name:literal => $function:ident ) => {
        {
            use $crate::service_dispatcher::{
                start_service_ctrl_dispatcher_raw,
                raw_service_main_wrapper,
            };

            unsafe extern "system" fn service_main_raw(argc: u32, argv: *mut *mut u16) {
                raw_service_main_wrapper(argc,argv,$function);
            }

            start_service_ctrl_dispatcher_raw($name, service_main_raw)
        }
    };
}

pub fn start_service_ctrl_dispatcher_raw(service_name: &str, service_main: RawServiceMain) -> Result<()> {
    unsafe {

        let service_name = to_wstr(&service_name);

        let service_table = [
            SERVICE_TABLE_ENTRYW{ lpServiceName: service_name.as_ptr(), lpServiceProc: Some(service_main) },
            SERVICE_TABLE_ENTRYW{ lpServiceName: null_mut(), lpServiceProc: None },
        ];

        if StartServiceCtrlDispatcherW(service_table.as_ptr()) != 0 {
            Ok(())
        } else {
            Err(Error::from_last())
        }
    }
}
