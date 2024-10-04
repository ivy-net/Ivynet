use ethers::{
    contract::ContractError,
    providers::{JsonRpcError, MiddlewareError as _, ProviderError},
    signers::WalletError,
    types::{Bytes, Chain, SignatureError, TryFromPrimitiveError},
    utils::hex::FromHexError,
};
use indicatif::style::TemplateError;
use thiserror::Error;
use tonic::Status;
use zip::result::ZipError;

use crate::{
    avs::{eigenda::EigenDAError, lagrange::LagrangeError},
    eigen::quorum::QuorumError,
    grpc::client::ClientError,
    rpc_management::IvyProvider,
};

#[derive(Debug, Error)]
pub enum IvyError {
    // ISSUE: Consider deprecating in favor of above.
    #[error(transparent)]
    StdIo(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    WalletError(#[from] WalletError),

    // TODO: Attempt to deprecate, see private_key_string to bytes methods.
    #[error(transparent)]
    HexError(#[from] FromHexError),

    #[error(transparent)]
    SignatureError(#[from] SignatureError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error(transparent)]
    DialogerError(#[from] dialoguer::Error),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    QuorumError(#[from] QuorumError),

    #[error(transparent)]
    EigenDAError(#[from] EigenDAError),

    #[error(transparent)]
    LagrangeError(#[from] LagrangeError),

    #[error(transparent)]
    ZipError(#[from] ZipError),

    #[error(transparent)]
    TemplateError(#[from] TemplateError),

    #[error(transparent)]
    TryFromChainError(#[from] TryFromPrimitiveError<Chain>),

    #[error(transparent)]
    GRPCError(#[from] Status),

    #[error(transparent)]
    SetupError(#[from] SetupError),

    #[error(transparent)]
    IoError(#[from] crate::io::IoError),

    #[error(transparent)]
    ConfigError(#[from] crate::config::ConfigError),

    #[error(transparent)]
    ProviderError(#[from] ProviderError),

    #[error(transparent)]
    ClientError(#[from] ClientError),

    #[error(
        "AVS {0} on chain {1} is currently running. Stop the AVS before using this operation."
    )]
    AvsRunningError(String, Chain),

    #[error("No valid key was found. Please create a key before trying again.")]
    NoKeyFoundError,

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

    #[error("Invalid tonic URI from string")]
    InvalidUri,

    #[error("No address field")]
    AddressFieldError,

    #[error("Folder inaccesible")]
    DirInaccessible,

    #[error("Unknown contract error")]
    UnknownContractError,

    #[error("Avs parse error: ensure the name of the requested AVS is valid")]
    InvalidAvsType(String),

    #[error("No AVS is initialized")]
    AvsNotInitializedError,

    #[error("Incorrect key type")]
    IncorrectKeyTypeError,

    #[error("Incorrect address format")]
    IncorrectAddressError,

    #[error("Can't parse to h160")]
    H160Error,

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

    #[error("Could not parse chain with name {0}")]
    ChainParseError(String),

    // TODO: The place where this is used should probably implement from for the parse() method
    // instead.
    #[error("Invalid address")]
    InvalidAddress,

    #[error("Log parse error {0}")]
    LogParseError(String),

    #[error(transparent)]
    BlsError(#[from] crate::bls::BlsKeyError),
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
