use ethers::{
    contract::ContractError,
    providers::{JsonRpcError, MiddlewareError as _, ProviderError},
    signers::WalletError,
    types::{Bytes, Chain, SignatureError, TryFromPrimitiveError},
    utils::hex::FromHexError,
};
use thiserror::Error;
use tonic::Status;
use zip::result::ZipError;

use crate::{
    docker::dockercmd::DockerError, eigen::quorum::QuorumError, grpc::client::ClientError,
    IvyProvider, IvyProviderError,
};

#[derive(Debug, Error)]
pub enum IvyError {
    // ISSUE: Consider deprecating in favor of above.
    #[error(transparent)]
    StdIo(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    SerdeYamlError(#[from] serde_yaml::Error),

    #[error(transparent)]
    SemverError(#[from] semver::Error),

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
    ZipError(#[from] ZipError),

    #[error(transparent)]
    GRPCError(#[from] Status),

    #[error(transparent)]
    SetupError(#[from] SetupError),

    #[error(transparent)]
    IoError(#[from] crate::io::IoError),

    #[error(transparent)]
    ConfigError(#[from] crate::config::ConfigError),

    #[error("Config type mismatch: expected {0}, found {1}")]
    ConfigMatchError(String, String),

    #[error(transparent)]
    ProviderError(#[from] ProviderError),

    #[error(transparent)]
    ClientError(#[from] ClientError),

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

    #[error("Unknown contract error")]
    UnknownContractError,

    #[error("Incorrect key type")]
    IncorrectKeyTypeError,

    #[error("Incorrect address format")]
    IncorrectAddressError,

    #[error(transparent)]
    TryFromPrimitiveError(#[from] TryFromPrimitiveError<Chain>),

    #[error("Can't parse to h160")]
    H160Error,

    #[error("Custom contract error")]
    ContractError(Bytes),

    #[error("JSON RPC Error {0}")]
    JsonRrcError(JsonRpcError),

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

    #[error(transparent)]
    IvyYamlError(#[from] crate::ivy_yaml::IvyYamlError),

    #[error(transparent)]
    DockerError(#[from] DockerError),

    #[error(transparent)]
    IvyWalletError(#[from] crate::wallet::IvyWalletError),

    #[error(transparent)]
    NodeConfigError(#[from] crate::avs::config::NodeConfigError),

    #[error(transparent)]
    KeychainError(#[from] crate::keychain::KeychainError),

    #[error(transparent)]
    EnvLineError(#[from] crate::env_parser::EnvLineError),

    #[error("Invalid docker-compose file: {0}")]
    InvalidDockerCompose(String),

    #[error(transparent)]
    DownloadError(#[from] crate::download::DownloadError),

    #[error(transparent)]
    SignerMiddlewareError(#[from] IvyProviderError),

    #[error(transparent)]
    NodeTypeError(#[from] crate::node_type::NodeTypeError),

    #[error("Docker Image Error")]
    DockerImageError,

    #[error(transparent)]
    TelemetryDispatchError(#[from] crate::telemetry::dispatch::TelemetryDispatchError),

    #[error("{0}")]
    CustomError(String),

    #[error("Not found")]
    NotFound,

    #[error("Signature error: {0}")]
    IvySignatureError(#[from] crate::signature::IvySigningError),

    #[error("Node find error, could not find node for name {0}")]
    NodeFindError(String),
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

impl From<String> for IvyError {
    fn from(e: String) -> Self {
        IvyError::CommandError(e)
    }
}
