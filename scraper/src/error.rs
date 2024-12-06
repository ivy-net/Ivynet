use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScraperError {
    #[error("Unknown error")]
    UnknownError,

    // #[error(transparent)]
    // UrlParseError(#[from] ivynet_core::grpc::client::UrlParseError),
    #[error(transparent)]
    JsonParseError(#[from] serde_json::Error),

    #[error(transparent)]
    ProviderError(#[from] ivynet_core::ethers::providers::ProviderError),

    #[error(transparent)]
    HexParsingError(#[from] ivynet_core::ethers::utils::hex::FromHexError),

    #[error(transparent)]
    WSContractError(
        #[from]
        ivynet_core::ethers::contract::ContractError<
            ivynet_core::ethers::providers::Provider<ivynet_core::ethers::providers::Ws>,
        >,
    ),

    #[error(transparent)]
    HTTPContractError(
        #[from]
        ivynet_core::ethers::contract::ContractError<
            ivynet_core::ethers::providers::Provider<ivynet_core::ethers::providers::Http>,
        >,
    ),

    #[error(transparent)]
    WSClientError(#[from] ivynet_core::ethers::providers::WsClientError),

    #[error(transparent)]
    TonicError(#[from] ivynet_core::grpc::tonic::Status),
}

pub type Result<T> = std::result::Result<T, ScraperError>;
