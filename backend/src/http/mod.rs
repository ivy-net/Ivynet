mod apidoc;
mod authorize;
mod client;
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
    let sender = sendgrid_api_key.map(Sender::new);

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
        .route("/organization/nodes", get(organization::nodes))
        .route("/organization/nodes/:id/metrics", get(organization::metrics))
        .route("/client/status", get(client::status))
        .route("/client/idle", get(client::idling))
        .route("/client/unhealthy", get(client::unhealthy))
        .route("/client/info/:id", get(client::info))
        .route("/client/node/:id", delete(client::delete))
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", apidoc::ApiDoc::openapi()),
        )
}
