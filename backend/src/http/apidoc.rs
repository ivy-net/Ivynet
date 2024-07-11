use utoipa::OpenApi;

use super::{super::db, authorize, organization};

#[derive(OpenApi)]
#[openapi(
    paths(
        authorize::authorize,
        authorize::check_invitation,
        authorize::set_password,
        organization::new,
        organization::get,
        organization::invite,
        organization::nodes
    ),
    components(
        schemas(
            authorize::AuthorizationCredentials,
            authorize::SetPasswordCredentials,
            authorize::AuthorizationResponse,
            organization::CreationResult,
            organization::CreationRequest,
            organization::InvitationResponse,
            organization::ConfirmationResponse,
            organization::InvitationRequest,
            db::Node
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
