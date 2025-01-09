#[derive(Debug, thiserror::Error)]
pub enum IngressError {
    #[error(transparent)]
    Tonic(#[from] ivynet_core::grpc::Status),

    #[error(transparent)]
    GRPCServerError(#[from] ivynet_core::grpc::server::ServerError),

    #[error(transparent)]
    GlobalTracingSetError(#[from] tracing::subscriber::SetGlobalDefaultError),

    #[error(transparent)]
    DatabaseError(#[from] db::error::DatabaseError),
}
