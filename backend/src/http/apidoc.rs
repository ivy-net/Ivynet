use utoipa::OpenApi;

use super::{super::db, authorize, client, organization};

#[derive(OpenApi)]
#[openapi(
    paths(
        authorize::authorize,
        authorize::check_invitation,
        authorize::set_password,
        authorize::forgot_password,
        organization::new,
        organization::get,
        organization::invite,
        organization::nodes,
        organization::confirm,
        client::status,
        client::idling,
        client::unhealthy,
        client::info,
        client::delete
    ),
    components(
        schemas(
            authorize::AuthorizationCredentials,
            authorize::SetPasswordCredentials,
            authorize::AuthorizationResponse,
            authorize::ForgotPasswordCredentials,
            organization::CreationResult,
            organization::CreationRequest,
            organization::InvitationResponse,
            organization::ConfirmationResponse,
            organization::InvitationRequest,
            db::Node,
            db::Role,
            client::Status,
            client::StatusReport,
            client::Info,
            client::InfoReport,
            client::Metrics
        ),
    ),
    tags(
        (
            name = "IvyNet Backend",
            description = "Full API for frontend communications with IvyNet backend"
        )
    )
)]
pub struct ApiDoc;
