mod apidoc;
mod authorize;
mod client;
mod organization;

use std::sync::Arc;

use crate::error::BackendError;

use axum::{
    http::{self, header, Method, StatusCode},
    routing::{get, options, post},
    Router,
};
use ivynet_core::grpc::client::Uri;
use sendgrid::v3::Sender;
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi as _;
use utoipa_swagger_ui::SwaggerUi;

use axum::{
    middleware::{self, Next},
    response::Response,
};

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

async fn add_headers(req: axum::http::Request<axum::body::Body>, next: Next) -> Response {
    println!("Received request: {:?}", req.method());
    let mut response = next.run(req).await;
    println!("Response status: {:?}", response.status());

    let headers = response.headers_mut();
    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, header::HeaderValue::from_static("*"));
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        header::HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        header::HeaderValue::from_static("Content-Type, Authorization"),
    );
    response
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
    tracing::info!("Starting HTTP server on port {port}");
    let sender = sendgrid_api_key.map(Sender::new);

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION])
        .allow_origin(Any);

    let app = Router::new()
        .layer(cors)
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
        .layer(middleware::from_fn(add_headers))
        .with_state(HttpState {
            pool,
            cache,
            sender,
            sender_email,
            root_url,
            org_verification_template,
            user_verification_template,
            pass_reset_template,
        })
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", apidoc::ApiDoc::openapi()),
        );

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{port}")).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_options() -> StatusCode {
    StatusCode::OK
}
