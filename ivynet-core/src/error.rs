use ethers::{
    contract::ContractError,
    middleware::{signer::SignerMiddlewareError, SignerMiddleware},
    providers::{Http, JsonRpcError, MiddlewareError as _, Provider},
    signers::WalletError,
    types::{Bytes, Chain, TryFromPrimitiveError},
    utils::hex::FromHexError,
};
use indicatif::style::TemplateError;
use thiserror::Error;
use tracing::subscriber::SetGlobalDefaultError;
use zip::result::ZipError;

use crate::{avs::eigenda::EigenDAError, eigen::quorum::QuorumError, wallet::IvyWallet};

#[derive(Debug, Error)]
pub enum IvyError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    GlobalTracingSetError(#[from] SetGlobalDefaultError),

    #[error(transparent)]
    WalletError(#[from] WalletError),

    #[error(transparent)]
    HexError(#[from] FromHexError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error(transparent)]
    ProviderError(#[from] SignerMiddlewareError<Provider<Http>, IvyWallet>),

    #[error(transparent)]
    DialogerError(#[from] dialoguer::Error),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    QuorumError(#[from] QuorumError),

    #[error(transparent)]
    EigenDAError(#[from] EigenDAError),

    #[error(transparent)]
    ZipError(#[from] ZipError),

    #[error(transparent)]
    TemplateError(#[from] TemplateError),

    #[error(transparent)]
    TryFromChainError(#[from] TryFromPrimitiveError<Chain>),

    #[error("Command failed with code:")]
    CommandError(String),

    #[error("Folder inaccesible")]
    DirInaccessible,

    #[error("Unknown contract error")]
    UnknownContractError,

    #[error("Avs parse error: ensure the name of the requested AVS is valid")]
    AvsParseError,

    #[error("Custom contract error")]
    ContractError(Bytes),

    #[error("JSON RPC Error {0}")]
    JsonRrcError(JsonRpcError),

    #[error("Download error")]
    DownloadError,

    #[error("Download interupted")]
    DownloadInt,

    #[error("No quorums to boot")]
    NoQuorums,

    #[error("Unknown network")]
    UnknownNetwork,

    #[error("Unimplemented")]
    Unimplemented,

    #[error(transparent)]
    SetupError(#[from] SetupError),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),

    #[error(transparent)]
    TomlDeError(#[from] toml::de::Error),
}

#[derive(Debug, Error)]
pub enum SetupError {
    #[error("No .env.example found")]
    NoEnvExample,
}

impl From<ContractError<SignerMiddleware<Provider<Http>, IvyWallet>>> for IvyError {
    fn from(value: ContractError<SignerMiddleware<Provider<Http>, IvyWallet>>) -> Self {
        match value {
            ContractError::Revert(bytes) => IvyError::ContractError(bytes),
            ContractError::MiddlewareError { e } => {
                if let Some(err) = e.as_error_response() {
                    IvyError::JsonRrcError(err.clone())
                } else {
                    IvyError::UnknownContractError
                }
            }
            ContractError::ProviderError { e } => {
                if let Some(err) = e.as_error_response() {
                    IvyError::JsonRrcError(err.clone())
                } else {
                    IvyError::UnknownContractError
                }
            }
            _ => IvyError::UnknownContractError,
        }
    }
}
