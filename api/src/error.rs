use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ivynet_docker_registry::registry::RegistryError;
use ivynet_grpc::server::ServerError;
use ivynet_heartbeat::{ClientId, MachineId, NodeId};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum BackendError {
    #[error(transparent)]
    Tonic(#[from] ivynet_grpc::Status),

    #[error(transparent)]
    DbError(#[from] ivynet_database::error::DatabaseError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    GlobalTracingSetError(#[from] tracing::subscriber::SetGlobalDefaultError),

    #[error(transparent)]
    MemcacheError(#[from] memcache::MemcacheError),

    #[error(transparent)]
    SendGridError(#[from] sendgrid::SendgridError),

    #[error(transparent)]
    GRPCServerError(#[from] ServerError),

    #[error(transparent)]
    MigrateError(#[from] sqlx::migrate::MigrateError),

    #[error("Condensed metrics not found: {0}")]
    CondensedMetricsNotFound(String),

    #[error("No running AVS found: {0}")]
    NoRunningAvsFound(String),

    #[error("No version information found: {0}")]
    NoVersionInfoFound(String),

    #[error("No internal ID attached to GRPC message")]
    NoInternalId,

    #[error("Bad credentials")]
    BadCredentials,

    #[error("Account already exists")]
    AccountExists,

    #[error("Insufficient priviledges")]
    InsufficientPriviledges,

    #[error("Already set")]
    AlreadySet,

    #[error("Invalid node id")]
    InvalidNodeId,

    #[error("Invalid AVS name")]
    InvalidAvs,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Malformed paramater: {0}: {1}")]
    MalformedParameter(String, String),

    #[error("Missing parameter: {0}")]
    MissingParameter(String),

    #[error("Invalid version")]
    InvalidVersion,

    #[error(transparent)]
    RegistryError(#[from] RegistryError),

    #[error("No valid node versions found")]
    NoVersionsFound,

    #[error(transparent)]
    NodeTypeError(#[from] ivynet_node_type::NodeTypeError),

    #[error("Bad id")]
    BadId,

    #[error("Invalid chain")]
    InvalidChain,

    #[error("Invalid data for set_avs_version")]
    InvalidSetAvsVersionData,

    #[error(transparent)]
    AlertError(#[from] ivynet_alerts::BitflagError),

    #[error("Alert not found for id: {0}")]
    AlertNotFound(Uuid),

    #[error("Alert not found for client id: {0}")]
    ClientHeartbeatAlertNotFound(ClientId),

    #[error("Alert not found for machine id: {0}")]
    MachineHeartbeatAlertNotFound(MachineId),

    #[error("Alert not found for node id: {0}")]
    NodeHeartbeatAlertNotFound(NodeId),
}

impl IntoResponse for BackendError {
    fn into_response(self) -> Response {
        match self {
            BackendError::InsufficientPriviledges => {
                (StatusCode::UNAUTHORIZED, "Account is not an admin".to_string()).into_response()
            }
            BackendError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "You are not authorized to access this function".to_string(),
            )
                .into_response(),
            BackendError::AccountExists => {
                (StatusCode::CONFLICT, "Account already exists".to_string()).into_response()
            }
            BackendError::InvalidNodeId => {
                (StatusCode::NOT_FOUND, "Node id not found".to_string()).into_response()
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", self)).into_response(),
        }
    }
}
