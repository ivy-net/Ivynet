mod apidoc;
mod authorize;
mod client;
mod organization;

use std::sync::Arc;

use crate::error::BackendError;

use axum::{
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    routing::{get, options, post},
    Router,
};
use ivynet_core::grpc::client::Uri;
use sendgrid::v3::Sender;
use sqlx::PgPool;
use url::Url;

use axum::{extract::Request, response::Response};

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
    tracing::info!("Starting HTTP server on port {port}");
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

    let app = app
        .clone()
        .with_state(state.clone())
        .layer(middleware::from_fn(check_origin))
        .layer(middleware::from_fn(add_headers));

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

async fn check_origin(mut request: Request, next: Next) -> Response {
    let headers = request.headers();

    println!("Headers: {:#?}", headers);

    let is_ivynet = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| Url::parse(s).ok())
        .map(|url| {
            println!("----------------------------URL FACTS-----------------------------");
            println!("URL: {:#?}", url);
            println!("Domain: {:#?}", url.domain());
            println!("Scheme: {:#?}", url.scheme());
            url.scheme() == "https" &&
                url.domain().map_or(false, |domain| {
                    domain == "ivynet.dev" || domain.ends_with(".ivynet.dev")
                })
        })
        .unwrap_or(false);

    println!("\n Is ivynet: {:#?} \n", is_ivynet);

    request.extensions_mut().insert(is_ivynet);

    let response = next.run(request).await;

    println!("Response {:#?}", response);

    response
}

async fn add_headers(req: Request, next: Next) -> Response {
    let is_ivynet = req.extensions().get::<bool>().copied().unwrap_or(false);
    let mut res = next.run(req).await;
    let headers = res.headers_mut();

    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Content-Type, Authorization"),
    );

    if is_ivynet {
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_static("https://*.ivynet.dev"),
        );
        headers.insert(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, HeaderValue::from_static("true"));
    } else {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    }

    res
}
