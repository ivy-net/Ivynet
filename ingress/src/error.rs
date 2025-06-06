#[derive(Debug, thiserror::Error)]
pub enum IngressError {
    #[error(transparent)]
    Tonic(#[from] ivynet_grpc::Status),

    #[error(transparent)]
    GRPCServerError(#[from] ivynet_grpc::server::ServerError),

    #[error(transparent)]
    GlobalTracingSetError(#[from] tracing::subscriber::SetGlobalDefaultError),

    #[error(transparent)]
    DatabaseError(#[from] ivynet_database::error::DatabaseError),

    #[error(transparent)]
    NotificationDispatcherError(#[from] ivynet_notifications::NotificationDispatcherError),
}
