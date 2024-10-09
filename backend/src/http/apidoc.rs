use utoipa::OpenApi;

use super::{super::db, authorize, avs, client, organization};

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
        client::metrics_condensed,
        client::metrics_all,
        client::delete,
        client::set_name,
        client::delete_node_data,
        client::delete_avs_node_data,
        client::get_all_node_data,
        client::get_node_data_for_avs,
        avs::get_node_data_for_avs,
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
            db::NodeData,
            db::AvsData,
            client::Status,
            client::StatusReport,
            client::Info,
            client::InfoReport,
            client::Metrics,
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
