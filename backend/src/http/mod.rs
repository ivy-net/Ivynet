mod apidoc;
mod authorize;
mod client;
mod info;
mod machine;
mod node;
mod organization;

use std::sync::Arc;

use crate::error::BackendError;

use axum::{
    http::Method,
    routing::{delete, get, post},
    Router,
};
use ivynet_core::grpc::client::Uri;
use sendgrid::v3::Sender;
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
        .route("/authorize", post(authorize::authorize))
        .route("/authorize/invitation/:id", get(authorize::check_invitation))
        .route("/authorize/forgot_password", post(authorize::forgot_password))
        .route("/authorize/set_password", post(authorize::set_password))
        .route("/organization", post(organization::new))
        .route("/organization/:id", get(organization::get))
        .route("/organization/invite", post(organization::invite))
        .route("/organization/confirm/:id", get(organization::confirm))
        .route("/organization/machines", get(organization::machines))
        .route("/organization/avses", get(organization::avses))
        .route("/client", get(client::client))
        .route("/client/:id", get(client::client_machines))
        .route("/machine", get(machine::machine))
        .route("/machine/status", get(machine::status))
        .route("/machine/idle", get(machine::idle))
        .route("/machine/unhealthy", get(machine::unhealthy))
        .route("/machine/healthy", get(machine::healthy))
        .route("/machine/:id/metrics", get(machine::metrics_condensed))
        .route("/machine/:id/metrics/all", get(machine::metrics_all))
        .route("/machine/:id/logs", get(machine::logs))
        .route("/machine/:id/data", get(machine::get_all_node_data))
        .route("/machine/:id/data", delete(machine::delete_machine_data))
        .route("/machine/:id", get(machine::info))
        .route("/machine/:id", post(machine::set_name))
        .route("/machine/:id", delete(machine::delete))
        .route("/avs", get(node::all_avs_info))
        .route("/avs/status", get(node::avs_status))
        .route("/avs/:id/:avs_name/:operator_id", delete(node::delete_avs_node_data))
        .route("info/avs/version/:avs", get(info::get_version_info))
        .route("info/avs/version", get(info::get_all_version_info))
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", apidoc::ApiDoc::openapi()),
        )
}
