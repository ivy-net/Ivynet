#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IvyError(#[from] ivynet_core::error::IvyError),

    #[error(transparent)]
    DialoguerError(#[from] dialoguer::Error),

    #[error(transparent)]
    GlobalTracingSetError(#[from] tracing::subscriber::SetGlobalDefaultError),

    #[error(transparent)]
    GRPCError(#[from] ivynet_core::grpc::Status),

    #[error(transparent)]
    GRPCClientError(#[from] ivynet_core::grpc::client::ClientError),

    #[error("Metadata Uri Not Found")]
    MetadataUriNotFoundError,
}
