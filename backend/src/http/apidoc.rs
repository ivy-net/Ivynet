use utoipa::OpenApi;

use super::{super::db, authorize, client, info, organization};
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
        organization::machines,
        organization::avses,
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
        client::get_all_node_data,
        info::get_version_info,
        info::get_all_version_info,
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
            db::Role,
            db::AvsVersionData,
            db::Avs,
            db::Machine,
            db::Client,
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
