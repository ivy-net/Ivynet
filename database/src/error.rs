#[derive(thiserror::Error, Debug)]
pub enum DatabaseError {
    #[error("Unknown error")]
    Unknown,

    #[error("Invalid chain provided")]
    InvalidChain,

    #[error("Invalid ID provided")]
    BadId,

    #[error("Can't parse pubkey: {0}")]
    CantParsePubKey(String),

    #[error("Operator key not found")]
    OperatorKeyNotFound,

    #[error("Failed to create operator key")]
    FailedToCreateOperatorKey,

    #[error("No valid node versions found")]
    NoVersionsFound,

    #[error("Invalid data for set_avs_version")]
    InvalidSetAvsVersionData,

    #[error(transparent)]
    GlobalSetEnv(#[from] tracing::subscriber::SetGlobalDefaultError),

    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),

    #[error(transparent)]
    MigrateError(#[from] sqlx::migrate::MigrateError),

    #[error(transparent)]
    NodeTypeError(#[from] ivynet_node_type::NodeTypeError),

    #[error(transparent)]
    RegistryError(#[from] ivynet_docker::RegistryError),
}
