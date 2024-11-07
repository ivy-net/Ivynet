use utoipa::OpenApi;

use super::{super::db, authorize, avs, client, organization};
use crate::data;

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
        client::client,
        client::logs,
        client::status,
        client::idling,
        client::unhealthy,
        client::healthy,
        client::info,
        client::metrics_condensed,
        client::metrics_all,
        client::delete,
        client::set_name,
        client::delete_node_data,
        client::delete_avs_node_data,
        client::get_all_node_data,
        client::get_node_data_for_avs,
        avs::get_version_info,
        avs::get_all_version_info
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
            db::AvsVersionData,
            db::metric::Metric,
            db::log::ContainerLog,
            db::log::LogLevel,
            data::NodeStatus,
            client::Status,
            client::StatusReport,
            client::Info,
            client::InfoReport,
            client::Metrics,
            client::NameChangeRequest,
            client::InfoReport,
            client::HardwareUsageInfo,
            client::HardwareInfoStatus,
            client::AvsInfo,
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
