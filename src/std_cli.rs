use std::fmt;
use serde::{Serialize, de::DeserializeOwned};
use structopt::{StructOpt,StructOptInternal};

use super::service_configuration;

#[derive(StructOpt,Debug)]
pub struct LoggingConfig {

    /// path to write log to
    #[structopt(long)]
    log_file: Option<String>,

    /// filter to use while logging
    #[structopt(long)]
    log_filter: Option<String>,
}

pub trait ServiceDetail {

    const SERVICE_IDENTIFIER: &'static str;
    const SERVICE_DISPLAY_NAME: &'static str;

    type Config: Serialize+DeserializeOwned+fmt::Debug;

    fn run_local(svc_config: Self::Config);
    fn run_as_service(log_config: LoggingConfig);

    fn install(svc_config: Self::Config, log_config: LoggingConfig) {
        tracing_subscriber::fmt::init();

        use crate::service_control_manager::*;

        let mut args = Vec::<&str>::with_capacity(5);

        args.push("run-as-service");

        if let Some(log_file) = &log_config.log_file {
            args.push("--log-file");
            args.push(log_file);
        }

        if let Some(log_filter) = &log_config.log_filter {
            args.push("--log-filter");
            args.push(log_filter);
        }

        tracing::trace!("args: {:?}", args);
        tracing::trace!("config: {:?}", svc_config);

        ServiceControlManager::open_local(Access::All)
            .expect("to open service control manager")
            .create_self_service_simple(
                Self::SERVICE_IDENTIFIER,
                Self::SERVICE_DISPLAY_NAME,
                &args,
                NETWORK_SERVICE,
            )
            .expect("to install self as service")
        ;

        service_configuration::save(Self::SERVICE_IDENTIFIER, &svc_config)
            .expect("while saving service configuration");
    }

    fn uninstall() {
        tracing_subscriber::fmt::init();
        open_service(Self::SERVICE_IDENTIFIER).delete().expect("to delete service");
    }

    fn start() {
        tracing_subscriber::fmt::init();
        open_service(Self::SERVICE_IDENTIFIER).start().expect("to start the service");
    }

    fn stop() {
        tracing_subscriber::fmt::init();
        open_service(Self::SERVICE_IDENTIFIER).stop().expect("to start the service");
    }
}



#[derive(StructOpt,Debug)]
pub enum Command<S:ServiceDetail> where <S as ServiceDetail>::Config: StructOpt {

    /// Execute service directly in this environment.
    Run(S::Config),

    /// install as a windows service
    Install{
        #[structopt(flatten)]
        svc_config: S::Config,

        #[structopt(flatten)]
        log_config: LoggingConfig,
    },

    /// uninstall as a windows service
    Uninstall,

    /// start the previously installed service
    Start,

    /// stop the previously installed and started service
    Stop,

    /// invoked by windows when started as a service [will fail if used elsewhere]
    RunAsService(LoggingConfig),
}

impl<S:ServiceDetail> Command<S> where <S as ServiceDetail>::Config: StructOpt + StructOptInternal + fmt::Debug {
    pub fn execute() {
        use Command::*;
        match Self::from_args() {
            Run(svc_config) => S::run_local(svc_config),
            RunAsService(log_config) => S::run_as_service(log_config),
            Install{svc_config,log_config} => S::install(svc_config,log_config),
            Uninstall => S::uninstall(),
            Start => S::start(),
            Stop => S::stop(),
        }
    }
}

impl LoggingConfig {
    pub fn init(self) {
        if let Some(log_file) = self.log_file {
            use tracing_subscriber::fmt::*;

            use std::fs::OpenOptions;

            let make_writer = move || OpenOptions::new().append(true).create(true).open(&log_file).expect("to open log file");

            let builder =
                Subscriber::builder()
                .with_ansi(false)
                .with_writer(make_writer)
                ;

            if let Some(log_filter) = self.log_filter {
                let builder = builder.with_env_filter(log_filter);
                builder.try_init().expect("to initializing tracing subscriber")
            } else {
                builder.try_init().expect("to initializing tracing subscriber")
            }
        }
    }
}

fn open_service(name: &str) -> crate::service_control_manager::Service {

    use crate::service_control_manager::*;

    let mut scm = ServiceControlManager::open_local(Access::All).expect("to open service control manager");

    scm.open_service(name).expect("to open service")
}
