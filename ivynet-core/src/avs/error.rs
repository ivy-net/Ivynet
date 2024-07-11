use ethers::types::{Chain, TryFromPrimitiveError};
use thiserror::Error as ThisError;

use crate::config;

#[derive(ThisError, Debug)]
pub enum AvsError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    TryFromChainError(#[from] TryFromPrimitiveError<Chain>),

    #[error(transparent)]
    ConfigError(#[from] config::ConfigError),
}
