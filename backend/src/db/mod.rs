use crate::error::BackendError;
use sqlx::{pool::PoolOptions, PgPool};

pub mod account;
pub mod avs;
pub mod avs_version;
pub mod client;
pub mod log;
pub mod machine;
pub mod metric;
pub mod operator_keys;
pub mod organization;
pub mod verification;

pub use account::{Account, Role};
pub use avs::Avs;
pub use avs_version::AvsVersionData;
pub use client::Client;
pub use machine::Machine;
pub use organization::Organization;

pub async fn configure(uri: &str, migrate: bool) -> Result<PgPool, BackendError> {
    let pool = PoolOptions::new().max_connections(5).connect(uri).await?;
    if migrate {
        sqlx::migrate!().run(&pool).await?;
    }
    Ok(pool)
}
