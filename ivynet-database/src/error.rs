use thiserror::Error;
use tonic::Status;

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

    #[error(transparent)]
    GRPCError(#[from] Status),

    #[error("Chain parse error: {0}")]
    ChainParseError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Notification type serialization error")]
    NotificationTypeSerializationError(#[from] serde_json::Error),

    #[error("Failed conversion: {0}")]
    FailedConversion(String),

    #[error("Failed to get count of metadata: {0}")]
    FailedMetadata(String),
}

impl From<DatabaseError> for Status {
    fn from(e: DatabaseError) -> Self {
        Self::from_error(Box::new(e))
    }
}
