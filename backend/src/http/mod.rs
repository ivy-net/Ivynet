mod apidoc;
mod authorize;
mod organization;

use std::sync::Arc;

use crate::error::BackendError;

use axum::{
    routing::{get, post},
    Router,
};
use ivynet_core::grpc::client::Uri;
use sendgrid::v3::Sender;
use sqlx::PgPool;
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
    port: u16,
) -> Result<(), BackendError> {
    tracing::info!("Starting HTTP server on port {port}");
    let sender = sendgrid_api_key.map(Sender::new);
    let app = Router::new()
        .route("/health", get(|| async { "alive" }))
        .route("/authorize", post(authorize::authorize))
        .route("/authorize/invitation/{id}", get(authorize::check_invitation))
        .route("/authorize/set_password", post(authorize::set_password))
        .route("/organization", post(organization::new))
        .route("/organization/{id}", get(organization::get))
        .route("/organization/invite", post(organization::invite))
        .route("/organization/confirm/{id}", get(organization::confirm))
        .route("/organization/nodes", get(organization::nodes))
        .with_state(HttpState {
            pool,
            cache,
            sender,
            sender_email,
            root_url,
            org_verification_template,
            user_verification_template,
        })
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", apidoc::ApiDoc::openapi()),
        );

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{port}")).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
