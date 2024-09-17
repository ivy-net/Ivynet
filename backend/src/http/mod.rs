mod apidoc;
mod authorize;
mod client;
mod organization;

use std::sync::Arc;

use crate::error::BackendError;

use axum::{
    http::{self, Method, StatusCode},
    routing::{get, options, post},
    Router,
};
use ivynet_core::grpc::client::Uri;
use sendgrid::v3::Sender;
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};
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
    let second_port = port + 1;
    tracing::info!("Starting HTTP server on port {port} and API service on {second_port}");
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

    let app = create_router();

    let cors_ivynet = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION])
        .allow_origin("https://ivynet.dev".parse::<http::HeaderValue>().unwrap())
        .allow_credentials(true);

    let cors_any_origin = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION])
        .allow_origin(Any);

    let app_ivynet = app.clone().layer(cors_ivynet).with_state(state.clone());
    let app_any = app.layer(cors_any_origin).with_state(state);

    let (ivynet_result, any_result) =
        tokio::join!(start_server(app_ivynet, port), start_server(app_any, second_port));

    ivynet_result?;
    any_result?;
    Ok(())
}

async fn start_server(app: Router, port: u16) -> Result<(), BackendError> {
    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{port}")).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn create_router() -> Router<HttpState> {
    Router::new()
        .route("/health", get(|| async { "alive" }))
        .route("/authorize", options(handle_options))
        .route("/authorize", post(authorize::authorize))
        .route("/authorize/invitation/{id}", get(authorize::check_invitation))
        .route("/authorize/forgot_password", post(authorize::forgot_password))
        .route("/authorize/set_password", post(authorize::set_password))
        .route("/organization", post(organization::new))
        .route("/organization/{id}", get(organization::get))
        .route("/organization/invite", post(organization::invite))
        .route("/organization/confirm/{id}", get(organization::confirm))
        .route("/organization/nodes", get(organization::nodes))
        .route("/organization/nodes/{id}/metrics", get(organization::metrics))
        .route("/client/status", get(client::status))
        .route("/client/idle", get(client::idling))
        .route("/client/unhealthy", get(client::unhealthy))
        .route("/client/info/{id}", get(client::info))
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", apidoc::ApiDoc::openapi()),
        )
}

async fn handle_options() -> StatusCode {
    StatusCode::OK
}
