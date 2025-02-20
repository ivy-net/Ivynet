mod alerts;
mod apidoc;
mod authorize;
mod client;
mod info;
mod machine;
mod node;
mod organization;
mod pubkey;

use std::sync::Arc;

use crate::error::BackendError;

use axum::{
    http::{Method, StatusCode},
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use ivynet_grpc::client::Uri;
use sendgrid::v3::Sender;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tracing::info;

use utoipa::OpenApi as _;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub struct HttpState {
    pub pool: Arc<PgPool>,
    pub cache: memcache::Client,
    pub sender: Option<Sender>,
    pub sender_email: Option<String>,
    pub root_url: Uri,
    pub org_verification_template: Option<String>,
    pub user_verification_template: Option<String>,
    pub pass_reset_template: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub async fn serve(
    pool: Arc<PgPool>,
    cache: memcache::Client,
    root_url: Uri,
    sendgrid_api_key: Option<String>,
    sender_email: Option<String>,
    org_verification_template: Option<String>,
    user_verification_template: Option<String>,
    pass_reset_template: Option<String>,
    port: u16,
) -> Result<(), BackendError> {
    info!("Starting HTTP server on port {port}");
    let sender = sendgrid_api_key.map(|key| Sender::new(key, None));

    let state = HttpState {
        pool,
        cache,
        sender,
        sender_email,
        root_url,
        org_verification_template,
        user_verification_template,
        pass_reset_template,
    };

    let app = create_router().with_state(state.clone()).layer(
        CorsLayer::very_permissive().allow_methods([
            Method::GET,
            Method::POST,
            Method::DELETE,
            Method::PUT,
            Method::OPTIONS,
        ]),
    );

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{port}")).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn create_router() -> Router<HttpState> {
    Router::new()
        .route("/health", get(|| async { "alive" }))
        .nest(
            "/authorize",
            Router::new()
                .route("/", post(authorize::authorize))
                .route("/invitation/:id", get(authorize::check_invitation))
                .route("/forgot_password", post(authorize::forgot_password))
                .route("/set_password", post(authorize::set_password)),
        )
        .nest(
            "/organization",
            Router::new()
                .route("/", post(organization::new))
                .route("/", get(organization::get_me))
                .route("/invite", post(organization::invite))
                .route("/confirm/:id", get(organization::confirm))
                .route("/machines", get(organization::machines))
                .route("/avses", get(organization::avses)),
        )
        .nest(
            "/client",
            Router::new()
                .route("/", get(client::client))
                .route("/:id", get(client::client_machines))
                .route("/:id/logs", get(client::client_logs)),
        )
        .nest(
            "/machine",
            Router::new()
                .route("/", get(machine::machine))
                .route("/status", get(machine::status))
                .route("/idle", get(machine::idle))
                .route("/unhealthy", get(machine::unhealthy))
                .route("/healthy", get(machine::healthy))
                .route("/delete", delete(machine::delete_machine))
                .route("/:machine_id/metrics/all", get(machine::metrics_all))
                .route("/:machine_id/metrics", get(machine::metrics_condensed))
                .route("/:machine_id/logs", get(machine::logs))
                .route("/:machine_id/info", get(machine::get_all_node_data))
                .route("/:machine_id/system_metrics", get(machine::system_metrics))
                .route("/:machine_id/node_type", put(machine::set_node_type))
                .route("/:machine_id", put(machine::update_avs))
                .route(
                    "/:machine_id",
                    get(machine::info)
                        .delete(machine::delete_avs_node_data)
                        .post(machine::set_name),
                ),
        )
        .nest(
            "/avs",
            Router::new()
                .route("/", get(node::all_avs_info))
                .route("/status", get(node::avs_status))
                .route("/active_set", get(node::avs_active_set)),
        )
        .nest(
            "/alerts",
            Router::new()
                .route("/active", get(alerts::active_alerts))
                .route("/history", get(alerts::alert_history))
                .route("/acknowledge", post(alerts::acknowledge_alert)),
        )
        .nest(
            "/info/avs",
            Router::new()
                .route("/version/:avs", get(info::get_version_info))
                .route("/version", get(info::get_all_version_info)),
        )
        .nest("/info/nodetypes", Router::new().route("/", get(info::get_node_types)))
        .nest(
            "/pubkey",
            Router::new()
                .route("/", get(pubkey::get_all_keys))
                .route("/", post(pubkey::create_key))
                .route("/", put(pubkey::update_key_name))
                .route("/", delete(pubkey::delete_key)),
        )
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", apidoc::ApiDoc::openapi()),
        )
        .fallback(handler_404_with_logging)
}

async fn handler_404_with_logging(uri: axum::http::Uri) -> (StatusCode, Json<Value>) {
    println!("404 Not Found for path: {}", uri.path());
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "Not Found",
            "message": "The requested resource does not exist"
        })),
    )
}
