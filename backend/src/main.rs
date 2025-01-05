use std::sync::Arc;

use clap::Parser as _;
use ivynet_backend::{config::Config, error::BackendError, grpc, http, telemetry::start_tracing};
use ivynet_core::grpc::{client::create_channel, database::database_client::DatabaseClient};
use tracing::error;

#[tokio::main]
async fn main() -> Result<(), BackendError> {
    let config = Config::parse();

    start_tracing(config.log_level)?;

    let cache = memcache::connect(config.cache_url.to_string())?;
    let database = Arc::new(DatabaseClient::new(
        create_channel(config.backend_uri, config.grpc_tls_ca).await?,
    ));

    let http_service = http::serve(
        database.clone(),
        cache,
        config.root_url,
        config.sendgrid_api_key,
        config.sendgrid_from,
        config.org_verification_template,
        config.user_verification_template,
        config.pass_reset_template,
        config.http_port,
    );
    let grpc_service =
        grpc::backend_serve(database, config.grpc_tls_cert, config.grpc_tls_key, config.grpc_port);

    tokio::select! {
        e = http_service => error!("HTTP server stopped. Reason {e:?}"),
        e = grpc_service => error!("Executor has stopped. Reason: {e:?}"),
    }

    Ok(())
}
