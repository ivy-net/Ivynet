use sqlx::{pool::PoolOptions, PgPool};

pub mod account;
pub mod alerts;
pub mod avs;
pub mod avs_active_set;
pub mod avs_version;
pub mod avs_version_hash;
pub mod client;
pub mod data;
pub mod error;
pub mod log;
pub mod machine;
pub mod metric;
pub mod operator_keys;
pub mod organization;
pub mod utils;
pub mod verification;

pub use account::{Account, Role};
pub use avs::Avs;
pub use avs_active_set::AvsActiveSet;
pub use avs_version::{AvsVersionData, DbAvsVersionData};
pub use avs_version_hash::AvsVersionHash;
pub use client::Client;
pub use machine::Machine;
pub use organization::Organization;

pub async fn configure(uri: &str, _migrate: bool) -> Result<PgPool, error::DatabaseError> {
    let pool = PoolOptions::new().max_connections(5).connect(uri).await?;
    // if migrate {
    //     sqlx::migrate!().run(&pool).await?;
    // }
    Ok(pool)
}
