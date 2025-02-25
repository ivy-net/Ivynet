use clap::Parser as _;
use db::configure;
use ivynet_ingress::{config::Config, error::IngressError, grpc};
use tracing::{error, warn, Level};
use tracing_subscriber::FmtSubscriber;

pub fn start_tracing(level: Level) -> Result<(), IngressError> {
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), IngressError> {
    if dotenvy::dotenv().is_err() {
        warn!("No .env file found, proceeding with shell defaults...")
    }

    let config = Config::parse();
    println!("{:?}", config.telegram_token);
    start_tracing(config.log_level)?;
    let pool = configure(&config.db_uri, false).await?;

    let grpc_service = grpc::backend_serve(
        pool.clone(),
        config.clone().into(),
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

    tokio::select! {
        e = grpc_service => error!("Executor has stopped. Reason: {e:?}"),
        e = events_service => error!("Events service has stopped. Reason: {e:?}"),
    }
    Ok(())
}
