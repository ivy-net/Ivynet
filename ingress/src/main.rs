use std::sync::Arc;

use clap::Parser as _;
use db::configure;
use ivynet_ingress::{config::Config, error::IngressError, grpc};
use tracing::{error, Level};
use tracing_subscriber::FmtSubscriber;

pub fn start_tracing(level: Level) -> Result<(), IngressError> {
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), IngressError> {
    let config = Config::parse();
    start_tracing(config.log_level)?;
    let pool = Arc::new(configure(&config.db_uri, false).await?);

    let grpc_service = grpc::backend_serve(
        pool.clone(),
        config.grpc_tls_cert,
        config.grpc_tls_key,
        config.grpc_port,
    );

    let events_service = grpc::events_serve(
        pool.clone(),
        config.events_tls_cert,
        config.events_tls_key,
        config.events_port,
    );

    let alerts_service =
        grpc::alerts_serve(pool, config.alerts_tls_cert, config.alerts_tls_key, config.alerts_port);

    tokio::select! {
        e = grpc_service => error!("Executor has stopped. Reason: {e:?}"),
        e = events_service => error!("Events service has stopped. Reason: {e:?}"),
        e = alerts_service => error!("Alerts service has stopped. Reason: {e:?}"),
    }
    Ok(())
}
