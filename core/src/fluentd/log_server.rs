use std::sync::Arc;

use crate::{
    error::IvyError,
    grpc::{
        backend::backend_client::BackendClient,
        messages::SignedLogs,
        tonic::{transport::Channel, Request},
    },
    signature::sign_string,
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
use tracing::debug;

pub type AppState = Arc<RwLock<LogServerState>>;

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
    let listener = tokio::net::TcpListener::bind("localhost:50051").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn post_log(
    State(state): State<AppState>,
    Json(log): Json<serde_json::Value>,
) -> Result<Json<bool>, LogServerError> {
    debug!("LOG | {:?}", log);
    let connection_wallet = state.read().await.connection_wallet.clone();
    let backend_client = &mut state.write().await.backend_client;
    send(&log.to_string(), &connection_wallet, backend_client).await?;
    Ok(true.into())
}

async fn send(
    logs: &str,
    wallet: &IvyWallet,
    client: &mut BackendClient<Channel>,
) -> Result<(), IvyError> {
    let signature = sign_string(logs, wallet).await?;
    client
        .logs(Request::new(SignedLogs { logs: logs.to_string(), signature: signature.to_vec() }))
        .await?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum LogServerError {
    #[error(transparent)]
    IvyError(#[from] crate::error::IvyError),
}

impl IntoResponse for LogServerError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error: {}", self))
            .into_response()
    }
}
