
use serde::{Serialize,Deserialize};
use structopt::StructOpt;
use tokio::sync::watch;
use winsvc::{
    std_cli::{Command,ServiceDetail,LoggingConfig},
    async_service_main::InitializationToken,
};

struct Service;

#[derive(StructOpt,Serialize,Deserialize,Debug)]
struct ServiceConfig{
  message: String
}

async fn run_for_a_while(config: ServiceConfig, mut running: watch::Receiver<bool>) {

    tracing::info!("entering {}", config.message);

    while *running.borrow() {
        running.changed().await.unwrap();
    }

    tracing::info!("exiting {}", config.message);

}

fn run_forever(config: ServiceConfig) {
    tracing_subscriber::fmt::init();
    let (_set_running,running) = tokio::sync::watch::channel(true);
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run_for_a_while(config, running))
}

async fn service_main(
    config: ServiceConfig,
    init: InitializationToken,
    running: watch::Receiver<bool>
) {
    init.complete();
    run_for_a_while(config, running).await
}

impl ServiceDetail for Service {

    const SERVICE_IDENTIFIER: &'static str = "winsvc-test-service-1";
    const SERVICE_DISPLAY_NAME: &'static str = "WinSvc Test Service 1";

    type Config = ServiceConfig;

    fn run_local(config: Self::Config) {
        run_forever(config);
    }

    fn run_as_service(log_config: LoggingConfig) {
        log_config.init();
        std::panic::set_hook(Box::new(|panic: &std::panic::PanicInfo<'_>| -> () {
            tracing::error!("panic: {}", panic);
        }));
        winsvc::async_service_dispatcher!{ "cpm-proxy" => service_main }
    }
}

fn main() {
  Command::<Service>::execute()
}
