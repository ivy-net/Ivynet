use crate::error::BackendError;
use sqlx::{pool::PoolOptions, PgPool};

pub mod account;
pub mod metric;
pub mod node;
pub mod organization;
pub mod verification;

pub use account::{Account, Role};
pub use node::Node;
pub use organization::Organization;

pub async fn configure(uri: &str, migrate: bool) -> Result<PgPool, BackendError> {
    let pool = PoolOptions::new().max_connections(5).connect(uri).await?;
    if migrate {
        sqlx::migrate!().run(&pool).await?;
    }
    Ok(pool)
}
