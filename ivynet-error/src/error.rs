use ethers::{
    contract::ContractError,
    providers::{JsonRpcError, MiddlewareError as _},
    types::Bytes,
};
use thiserror::Error;

use crate::IvyProvider;

#[derive(Debug, Error)]
pub enum IvyError {
    #[error("Command failed with code:")]
    CommandError(String),

    #[error("Unknown contract error")]
    UnknownContractError,

    #[error("Custom contract error")]
    ContractError(Bytes),

    #[error("JSON RPC Error {0}")]
    JsonRrcError(JsonRpcError),
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
