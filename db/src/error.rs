use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),

    #[error("Can't parse pubkey: {0}")]
    CantParsePubKey(String),

    #[error("Invalid chain")]
    InvalidChain,

    #[error("Operator key not found")]
    OperatorKeyNotFound,

    #[error("Failed to create operator key")]
    FailedToCreateOperatorKey,

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Bad id")]
    BadId,

    #[error("No valid node versions found")]
    NoVersionsFound,

    #[error("Data integrity error")]
    DataIntegrityError(String),

    #[error("Local build only node type")]
    LocalOnlyNode,
}
