use std::sync::Arc;

use clap::Parser;
use ethers::types::Chain;
use sqlx::{pool::PoolOptions, PgPool};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use ivynet_database::{config::Config, error::DatabaseError, utils};

#[tokio::main]
async fn main() -> Result<(), DatabaseError> {
    let config = Config::parse();
    start_tracing(config.log_level)?;

    let pool = Arc::new(configure(&config.db_uri, config.migrate).await?);
    if let Some(organization) = config.add_organization {
        Ok(utils::add_account(&pool, &organization).await?)
    } else if let Some(avs_data) = config.set_avs_version {
        Ok(utils::set_avs_version(&pool, &avs_data).await?)
    } else if let Some(avs_data) = config.add_avs_version_hash {
        Ok(utils::add_version_hash(&pool, &avs_data).await?)
    } else if let Some(avs_data) = config.set_breaking_change_version {
        Ok(utils::set_breaking_change_version(&pool, &avs_data).await?)
    } else if config.add_node_version_hashes {
        Ok(utils::add_node_version_hashes(&pool).await?)
    } else if config.update_node_data_versions {
        utils::update_node_data_versions(&pool, &Chain::Mainnet).await?;
        utils::update_node_data_versions(&pool, &Chain::Holesky).await?;
        return Ok(());
    } else {
        // TODO: Run service
        Ok(())
    }
}

fn start_tracing(level: Level) -> Result<(), DatabaseError> {
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

async fn configure(uri: &str, migrate: bool) -> Result<PgPool, DatabaseError> {
    let pool = PoolOptions::new().max_connections(5).connect(uri).await?;
    if migrate {
        sqlx::migrate!().run(&pool).await?;
    }
    Ok(pool)
}
