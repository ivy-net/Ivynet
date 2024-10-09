use axum::{
    routing::{get, post},
    Json, Router,
};
use tracing::debug;

pub async fn serve_log_server() -> Result<(), LogServerErrror> {
    let app =
        Router::new().route("/post_log", post(post_log)).route("/", get(|| async { "Alive" }));
    let listener = tokio::net::TcpListener::bind("localhost:50051").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn post_log(Json(log): Json<serde_json::Value>) {
    debug!("log: {:?}", log);
}

// #[derive(Debug, Deserialize, Serialize)]
// struct Log {
//     container_name: String,
//     source: String,
//     log: serde_json::Value,
// }

#[derive(Debug, thiserror::Error)]
pub enum LogServerErrror {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}
