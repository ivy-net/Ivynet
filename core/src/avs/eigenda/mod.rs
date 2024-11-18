use crate::eigen::quorum::QuorumType;
use core::str;
use thiserror::Error as ThisError;
use tracing::error;

mod config;
mod contracts;

pub use config::*;

pub const EIGENDA_PATH: &str = ".eigenlayer/eigenda";
pub const EIGENDA_SETUP_REPO: &str =
    "https://github.com/ivy-net/eigenda-operator-setup/archive/refs/heads/master.zip";

#[derive(ThisError, Debug)]
pub enum EigenDAError {
    #[error("Boot script failed: {0}")]
    ScriptError(String),
    #[error("Not eligible for Quorum: {0}")]
    QuorumValidationError(QuorumType),
    #[error("Failed to download resource: {0}")]
    DownloadFailedError(String),
    #[error("No bootable quorums found. Please check your operator shares.")]
    NoBootableQuorumsError,
}
