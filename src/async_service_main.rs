
use std::sync::{Arc,Mutex,atomic::{Ordering::Relaxed,AtomicIsize}};
use std::future::Future;
use std::time::Duration;
use tokio::sync::watch;

use crate::service_dispatcher::*;

pub struct InitializationToken(Arc<Mutex<ServiceStatus>>);

impl<'a> InitializationToken {
    /// if initialization is going to take more than *a second* call this
    /// periodically as progress is made
    pub fn still_starting(&mut self, wait_hint: Duration) {
        self.0.lock().unwrap().starting(wait_hint).unwrap();
    }

    /// notify system that service initialization is complete
    pub fn complete(self) {
        self.0.lock().unwrap().running().unwrap()
    }
}

pub unsafe fn raw_async_service_main_wrapper<T,U,V>(name: &str, _argc: u32, _argv: *mut *mut u16, function: T) where
    T: Copy+FnOnce(V,InitializationToken,watch::Receiver<bool>)->U,
    U: Future<Output=()>,
    V: serde::de::DeserializeOwned,
{
    const RUN_LEVEL_STOPPED : isize = 0;
    const RUN_LEVEL_PAUSED  : isize = 1;
    const RUN_LEVEL_STARTED : isize = 2;

    let requested_state = Arc::new(AtomicIsize::new(RUN_LEVEL_STARTED));

    let (set_running,running) = watch::channel(true);
    let (set_paused,mut paused) = watch::channel(false);

    let handler = {
        let requested_state = requested_state.clone();
        move |event: ServiceEvent| -> ServiceEventResult {
            tracing::info!("service control request: {:?}", event);
            match event {
                ServiceEvent::Pause => {
                    requested_state.store(RUN_LEVEL_PAUSED, Relaxed);
                    if !*set_paused.borrow() {
                        set_paused.send(true).unwrap();
                    }
                    if *set_running.borrow() {
                        set_running.send(false).unwrap();
                    }
                    Ok(())
                },
                ServiceEvent::Continue => {
                    requested_state.store(RUN_LEVEL_STARTED, Relaxed);
                    if !*set_running.borrow() {
                        set_running.send(true).unwrap();
                    }
                    if *set_paused.borrow() {
                        set_paused.send(false).unwrap();
                    }
                    Ok(())
                },
                ServiceEvent::Stop => {
                    requested_state.store(RUN_LEVEL_STOPPED, Relaxed);
                    set_running.send(false).unwrap();
                    set_paused.send(false).unwrap();
                    Ok(())
                },
                _ => Err(SERVICE_EVENT_NOT_IMPLEMENTED)
            }
        }
    };

    let status = Arc::new(Mutex::new(register_service_ctrl_handler(name, handler).unwrap()));

    loop {

        match requested_state.load(Relaxed) {
            RUN_LEVEL_STOPPED => {
                status.lock().unwrap().stopped().unwrap();
                break;
            },
            RUN_LEVEL_PAUSED => {
                tracing::trace!("entering paused state");
                status.lock().unwrap().paused().unwrap();
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build().unwrap()
                    .block_on(async {
                        while *paused.borrow() {
                            paused.changed().await.unwrap();
                        }
                    });
                tracing::trace!("exiting paused state");
            },
            RUN_LEVEL_STARTED => {
                tracing::trace!("entering started state");
                let config = crate::service_configuration::load::<V>(name).unwrap();
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(function(config,InitializationToken(status.clone()), running.clone()));
                tracing::trace!("exiting started state");
            },
            _ => panic!()
        }
    }
}

#[macro_export]
macro_rules! async_service_dispatcher {
    ( $name:literal => $function:ident ) => {
        {
            use $crate::service_dispatcher::{
                start_service_ctrl_dispatcher_raw,
            };
            use $crate::async_service_main::{
                raw_async_service_main_wrapper,
            };

            unsafe extern "system" fn service_main_raw(argc: u32, argv: *mut *mut u16) {
                raw_async_service_main_wrapper($name, argc, argv, $function);
            }

            start_service_ctrl_dispatcher_raw($name, service_main_raw).unwrap()
        }
    };
}
