#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IvyError(#[from] ivynet_core::error::IvyError),

    #[error(transparent)]
    DialoguerError(#[from] dialoguer::Error),

    #[error(transparent)]
    TracingFilterParseError(#[from] tracing_subscriber::filter::ParseError),

    #[error(transparent)]
    GRPCError(#[from] ivynet_core::grpc::Status),

    #[error("Metadata Uri Not Found")]
    MetadataUriNotFoundError,
}
