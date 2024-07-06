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
use tonic::Status;
use tracing::subscriber::SetGlobalDefaultError;
use zip::result::ZipError;

use crate::{avs::eigenda::EigenDAError, eigen::quorum::QuorumError, rpc_management::IvyProvider, wallet::IvyWallet};

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

    #[error(transparent)]
    GRPCError(#[from] Status),

    #[error("AVS already started")]
    AvsAlreadyLoadedError,

    #[error("AVS already started")]
    AvsNotLoadedError,

    #[error("Chain not supported {0}")]
    ChainNotSupportedError(Chain),

    #[error("Command failed with code:")]
    CommandError(String),

    #[error("GRPC server error")]
    GRPCServerError,

    #[error("GRPC client error")]
    GRPCClientError,

    #[error("Folder inaccesible")]
    DirInaccessible,

    #[error("Unknown contract error")]
    UnknownContractError,

    #[error("Avs parse error: ensure the name of the requested AVS is valid")]
    InvalidAvsType(String),

    #[error("No AVS is initialized")]
    AvsNotInitializedError,

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

    #[error("Malformed config found, ensure ivynet setup was run correctly")]
    MalformedConfigError,

    #[error("IvyWallet identity key not found")]
    IdentityKeyError,

    #[error("No keyfile password found")]
    KeyfilePasswordError,

    #[error("Unknown network")]
    UnknownNetwork,

    #[error("Unimplemented")]
    Unimplemented,

    // TODO: The place where this is used should probably implement from for the parse() method
    // instead.
    #[error("Invalid address")]
    InvalidAddress,

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

impl From<ContractError<IvyProvider>> for IvyError {
    fn from(value: ContractError<IvyProvider>) -> Self {
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

impl From<IvyError> for Status {
    fn from(e: IvyError) -> Self {
        Self::from_error(Box::new(e))
    }
}
