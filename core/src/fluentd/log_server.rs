use std::sync::Arc;

use crate::{
    error::IvyError,
    grpc::{backend::backend_client::BackendClient, tonic::transport::Channel},
    wallet::IvyWallet,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use tokio::sync::RwLock;

pub type AppState = Arc<RwLock<LogServerState>>;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct LogServerState {
    backend_client: BackendClient<Channel>,
    connection_wallet: IvyWallet,
}

pub async fn serve_log_server(
    client: BackendClient<Channel>,
    connection_wallet: IvyWallet,
) -> Result<(), IvyError> {
    let state = Arc::new(RwLock::new(LogServerState { backend_client: client, connection_wallet }));
    let app = Router::new()
        .route("/health", get(|| async { "Alive" }))
        .route("/post_log", post(post_log))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:50051").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn post_log(
    State(_state): State<AppState>,
    Json(_log): Json<serde_json::Value>,
) -> Result<Json<bool>, LogServerError> {
    // TODO: This server is not valid anymore, right?
    // let connection_wallet = state.read().await.connection_wallet.clone();
    // let backend_client = &mut state.write().await.backend_client;
    // send(&log.to_string(), &connection_wallet, backend_client).await?;
    Ok(true.into())
}

#[allow(dead_code)]
async fn send(
    _logs: &str,
    _wallet: &IvyWallet,
    _client: &mut BackendClient<Channel>,
) -> Result<(), IvyError> {
    // TODO: This is not gonna be here either, right?
    // let signature = sign_string(logs, wallet)?;
    // debug!("Log send | Signature: {:?} Logs: {:?}", signature, logs);
    // client
    //     .logs(Request::new(SignedLogs { logs: logs.to_string(), signature: signature.to_vec() }))
    //     .await?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum LogServerError {
    #[error(transparent)]
    IvyError(#[from] crate::error::IvyError),

    #[error("Invalid log JSON")]
    InvalidJson,
}

impl IntoResponse for LogServerError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error: {}", self))
            .into_response()
    }
}
