use std::sync::Arc;

use clap::Parser as _;
use ivynet_backend::{
    config::Config,
    db::{self, avs_version::DbAvsVersionData, configure},
    error::BackendError,
    grpc, http,
    telemetry::start_tracing,
};
use ivynet_core::{avs::names::AvsName, utils::try_parse_chain};
use semver::Version;
use sqlx::PgPool;
use tracing::error;

#[tokio::main]
async fn main() -> Result<(), BackendError> {
    let config = Config::parse();

    start_tracing(config.log_level)?;

    let pool = Arc::new(configure(&config.db_uri, config.migrate).await?);

    if let Some(organization) = config.add_organization {
        Ok(add_account(&pool, &organization).await?)
    } else if let Some(avs_data) = config.set_avs_version {
        Ok(set_avs_version(&pool, &avs_data).await?)
    } else {
        let cache = memcache::connect(config.cache_url.to_string())?;
        let http_service = http::serve(
            pool.clone(),
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
            grpc::serve(pool, config.grpc_tls_cert, config.grpc_tls_key, config.grpc_port);

        tokio::select! {
            e = http_service => error!("HTTP server stopped. Reason {e:?}"),
            e = grpc_service => error!("Executor has stopped. Reason: {e:?}"),
        }

        Ok(())
    }
}

async fn add_account(pool: &PgPool, org: &str) -> Result<(), BackendError> {
    let org_data = org.split('/').collect::<Vec<_>>();
    if org_data.len() == 2 {
        let cred_data = org_data[0].split(':').collect::<Vec<_>>();
        if cred_data.len() == 2 {
            println!("Creating organization {} with user {}", org_data[1], cred_data[0]);
            let org = db::Organization::new(pool, org_data[1], true).await?;
            org.attach_admin(pool, cred_data[0], cred_data[1]).await?;
        }
    }
    Ok(())
}

async fn set_avs_version(pool: &sqlx::PgPool, avs_data: &str) -> Result<(), BackendError> {
    let avs_data = avs_data.split(':').collect::<Vec<_>>();
    let name = AvsName::try_from(avs_data[0]).map_err(|_| BackendError::InvalidAvs)?;
    let chain = try_parse_chain(avs_data[1]).expect("Cannot parse chain");
    let version = Version::parse(avs_data[2]).expect("Cannot parse version");

    println!("Setting version {:?} for avs {:?} on {:?}", version, name, chain);
    DbAvsVersionData::set_avs_version(pool, &name, &chain, &version).await?;
    Ok(())
}
