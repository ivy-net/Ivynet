use std::sync::Arc;

use clap::Parser as _;
use ivynet_backend::{
    config::Config,
    db::{self, configure},
    error::BackendError,
    grpc, http,
    telemetry::start_tracing,
};
use sqlx::PgPool;
use tracing::{error, warn};

#[tokio::main]
async fn main() -> Result<(), BackendError> {
    let config = Config::parse();

    start_tracing(config.log_level)?;

    let pool = Arc::new(configure(&config.db_uri).await?);
    let cache = memcache::connect(config.cache_url.to_string())?;

    // If there's a test account set, prune all accounts and set this one
    set_test_account(&pool, config.test_account).await?;

    let http_service = http::serve(
        pool.clone(),
        cache,
        config.root_url,
        config.sendgrid_api_key,
        config.sendgrid_from,
        config.org_verification_template,
        config.user_verification_template,
        config.http_port,
    );
    let grpc_service = grpc::serve(
        pool,
        config.grpc_tls_cert,
        config.grpc_tls_key,
        config.grpc_port,
    );

    tokio::select! {
        _ = http_service => error!("HTTP server stopped"),
        _ = grpc_service => error!("Executor has stopped"),
    }

    Ok(())
}

async fn set_test_account(pool: &PgPool, account: Option<String>) -> Result<(), BackendError> {
    if let Some(credentials) = account {
        let cred_data = credentials.split(':').collect::<Vec<_>>();
        if cred_data.len() == 2 {
            warn!(
                "Replacing all accounts with testing one {} pass {}",
                cred_data[0], cred_data[1]
            );
            db::Organization::purge(pool).await?;
            let org = db::Organization::new(pool, "Test Organization", true).await?;
            org.attach_admin(pool, cred_data[0], cred_data[1]).await?;
        }
    }
    Ok(())
}
