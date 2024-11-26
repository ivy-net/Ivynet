use crate::data::{machine_data, node_data};
use utoipa::OpenApi;

use super::{super::db, authorize, client, info, machine, node, organization, pubkey};

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
        client::client_machines,
        info::get_version_info,
        info::get_all_version_info,
        machine::machine,
        machine::status,
        machine::idle,
        machine::unhealthy,
        machine::healthy,
        machine::metrics_condensed,
        machine::metrics_all,
        machine::logs,
        machine::get_all_node_data,
        machine::delete_machine,
        machine::delete_avs_node_data,
        machine::info,
        node::all_avs_info,
        node::avs_status,
        organization::add_operator_key,
        pubkey::get_all_keys,
        pubkey::create_key,
        pubkey::update_key_name,
        pubkey::delete_key,
        machine::update_avs,
        machine::set_name,
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
            node_data::NodeStatusReport,
            machine_data::MachineInfoReport,
            machine_data::MachineStatusReport,
            organization::AddOperatorKeyRequest,
            pubkey::CreateKeyRequest,
            pubkey::UpdateKeyNameRequest,
            pubkey::DeleteKeyRequest,
            db::operator_keys::OperatorKey,
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
