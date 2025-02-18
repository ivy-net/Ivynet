use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScraperError {
    #[error("Unknown error")]
    UnknownError,

    #[error(transparent)]
    JsonParseError(#[from] serde_json::Error),

    #[error(transparent)]
    ProviderError(#[from] ivynet_error::ethers::providers::ProviderError),

    #[error(transparent)]
    HexParsingError(#[from] ivynet_error::ethers::utils::hex::FromHexError),

    #[error(transparent)]
    WSContractError(
        #[from]
        ivynet_error::ethers::contract::ContractError<
            ivynet_error::ethers::providers::Provider<ivynet_error::ethers::providers::Ws>,
        >,
    ),

    #[error(transparent)]
    HTTPContractError(
        #[from]
        ivynet_error::ethers::contract::ContractError<
            ivynet_error::ethers::providers::Provider<ivynet_error::ethers::providers::Http>,
        >,
    ),

    #[error(transparent)]
    WSClientError(#[from] ivynet_error::ethers::providers::WsClientError),

    #[error(transparent)]
    TonicError(#[from] ivynet_grpc::tonic::Status),
}

pub type Result<T> = std::result::Result<T, ScraperError>;
