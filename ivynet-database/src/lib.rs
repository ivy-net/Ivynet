use sqlx::{pool::PoolOptions, PgPool};

pub mod account;
pub mod alerts;
pub mod avs;
pub mod avs_active_set;
pub mod avs_version;
pub mod avs_version_hash;
pub mod client;
pub mod client_log;
pub mod data;
pub mod eigen_avs_metadata;
pub mod error;
pub mod log;
pub mod machine;
pub mod metric;
pub mod notification_settings;
pub mod operator_keys;
pub mod organization;
pub mod service_settings;
pub mod utils;
pub mod verification;

pub use account::{Account, Role};
pub use avs::Avs;
pub use avs_active_set::AvsActiveSet;
pub use avs_version::{AvsVersionData, DbAvsVersionData};
pub use avs_version_hash::AvsVersionHash;
pub use client::Client;
pub use machine::Machine;
pub use notification_settings::NotificationSettings;
pub use organization::Organization;
pub use service_settings::ServiceSettings;

pub async fn configure(uri: &str, _migrate: bool) -> Result<PgPool, error::DatabaseError> {
    let pool = PoolOptions::new().max_connections(5).connect(uri).await?;
    // if migrate {
    //     sqlx::migrate!().run(&pool).await?;
    // }
    Ok(pool)
}
