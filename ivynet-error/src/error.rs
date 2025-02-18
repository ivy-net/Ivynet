use ethers::{
    contract::ContractError,
    providers::{JsonRpcError, MiddlewareError as _, ProviderError},
    signers::WalletError,
    types::{Bytes, Chain, SignatureError, TryFromPrimitiveError},
    utils::hex::FromHexError,
};
use ivynet_docker::dockercmd::DockerError;
use ivynet_grpc::client::ClientError;
use ivynet_signer::IvyWalletError;
use thiserror::Error;
use zip::result::ZipError;

use crate::{IvyProvider, IvyProviderError};

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
    ZipError(#[from] ZipError),

    #[error(transparent)]
    IoError(#[from] ivynet_io::IoError),

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
    BlsError(#[from] ivynet_signer::bls::BlsKeyError),

    #[error(transparent)]
    DockerError(#[from] DockerError),

    #[error(transparent)]
    IvyWalletError(#[from] IvyWalletError),

    #[error(transparent)]
    KeychainError(#[from] ivynet_signer::keychain::KeychainError),

    #[error("Invalid docker-compose file: {0}")]
    InvalidDockerCompose(String),

    #[error(transparent)]
    SignerMiddlewareError(#[from] IvyProviderError),

    #[error(transparent)]
    NodeTypeError(#[from] ivynet_node_type::NodeTypeError),

    #[error("Docker Image Error")]
    DockerImageError,

    #[error("{0}")]
    CustomError(String),

    #[error("Not found")]
    NotFound,

    #[error("Signature error: {0}")]
    IvySignatureError(#[from] ivynet_signer::sign_utils::IvySigningError),

    #[error("Node find error, could not find node for name {0}")]
    NodeFindError(String),

    #[error(transparent)]
    DockerStreamError(#[from] ivynet_docker::dockerapi::DockerStreamError),
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

impl From<String> for IvyError {
    fn from(e: String) -> Self {
        IvyError::CommandError(e)
    }
}
