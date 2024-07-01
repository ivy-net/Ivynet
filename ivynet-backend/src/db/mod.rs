use crate::error::BackendError;
use sqlx::{pool::PoolOptions, PgPool};

pub mod account;
pub mod node;
pub mod organization;
pub mod verification;

pub use account::{Account, Role};
pub use node::Node;
pub use organization::Organization;

pub async fn configure(uri: &str) -> Result<PgPool, BackendError> {
    Ok(PoolOptions::new().max_connections(5).connect(uri).await?)
}
