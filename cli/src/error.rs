#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ConfigError(#[from] ivynet_core::config::ConfigError),

    #[error(transparent)]
    IvyError(#[from] ivynet_core::error::IvyError),

    #[error(transparent)]
    ServerError(#[from] ivynet_core::grpc::server::ServerError),

    #[error(transparent)]
    DialoguerError(#[from] dialoguer::Error),

    #[error(transparent)]
    GlobalTracingSetError(#[from] tracing::subscriber::SetGlobalDefaultError),

    #[error(transparent)]
    GRPCError(#[from] ivynet_core::grpc::Status),

    #[error(transparent)]
    GRPCClientError(#[from] ivynet_core::grpc::client::ClientError),

    #[error("No AVS selected for log viewing. Please select an AVS first, or specify the AVS and chain you would like to view logs for.")]
    NoAvsSelectedLogError,

    #[error("Metadata Uri Not Found")]
    MetadataUriNotFoundError,

    #[error(transparent)]
    StdIo(#[from] std::io::Error),

    #[error("Invalid selection")]
    InvalidSelection,
}
