use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use ivynet_heartbeat::{
    alerts::{
        ClientHeartbeatAlert, ClientHeartbeatAlertHistorical, MachineHeartbeatAlert,
        MachineHeartbeatAlertHistorical, NodeHeartbeatAlert, NodeHeartbeatAlertHistorical,
    },
    ClientId, MachineId, NodeId,
};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{error::BackendError, http::authorize};

use super::HttpState;

#[derive(Debug, Clone, Copy, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct PaginationParams {
    pub limit: i64,
    pub offset: i64,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self { limit: 100, offset: 0 }
    }
}

#[derive(Debug, Clone, Copy, ToSchema, Deserialize, utoipa::IntoParams)]
pub struct ClientAlertParams {
    pub client_id: ClientId,
}

#[derive(Debug, Clone, Copy, ToSchema, Deserialize, utoipa::IntoParams)]
pub struct MachineAlertParams {
    pub machine_id: MachineId,
}

#[derive(Debug, Clone, ToSchema, Deserialize, utoipa::IntoParams)]
pub struct NodeAlertParams {
    pub node_id: NodeId,
}

/// Get all active client heartbeat alerts
#[utoipa::path(
    get,
    path = "/alerts/heartbeat/client/active",
    responses(
        (status = 200, body = [ClientHeartbeatAlert]),
        (status = 404)
    )
)]
pub async fn client_active_alerts(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<ClientHeartbeatAlert>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alerts =
        ClientHeartbeatAlert::get_by_organization_id(&state.pool, account.organization_id).await?;
    Ok(Json(alerts))
}

/// Get historical client heartbeat alerts
#[utoipa::path(
    get,
    path = "/alerts/heartbeat/client/history",
    params(PaginationParams),
    responses(
        (status = 200, body = [ClientHeartbeatAlertHistorical]),
        (status = 404)
    )
)]
pub async fn client_alert_history(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Vec<ClientHeartbeatAlertHistorical>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alerts = ClientHeartbeatAlertHistorical::get_by_organization_id(
        &state.pool,
        account.organization_id,
        params.limit,
        params.offset,
    )
    .await?;
    Ok(Json(alerts))
}

/// Acknowledge a client heartbeat alert and move it to the historical table - used for deprecated
/// clients
#[utoipa::path(
    post,
    path = "/alerts/heartbeat/client/acknowledge",
    params(ClientAlertParams),
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn acknowledge_client_alert(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<ClientAlertParams>,
) -> Result<Json<()>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alert_id = params.client_id;
    let alert = ClientHeartbeatAlert::get(&state.pool, params.client_id)
        .await?
        .ok_or(BackendError::ClientHeartbeatAlertNotFound(alert_id))?;
    if alert.organization_id != account.organization_id {
        return Err(BackendError::ClientHeartbeatAlertNotFound(alert_id));
    }
    ClientHeartbeatAlert::resolve(&state.pool, params.client_id).await?;
    Ok(Json(()))
}

/// Get all active machine heartbeat alerts
#[utoipa::path(
    get,
    path = "/alerts/heartbeat/machine/active",
    responses(
        (status = 200, body = [MachineHeartbeatAlert]),
        (status = 404)
    )
)]
pub async fn machine_active_alerts(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<MachineHeartbeatAlert>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alerts =
        MachineHeartbeatAlert::get_by_organization_id(&state.pool, account.organization_id).await?;
    Ok(Json(alerts))
}

/// Get historical machine heartbeat alerts
#[utoipa::path(
    get,
    path = "/alerts/heartbeat/machine/history",
    params(PaginationParams),
    responses(
        (status = 200, body = [MachineHeartbeatAlertHistorical]),
        (status = 404)
    )
)]
pub async fn machine_alert_history(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Vec<MachineHeartbeatAlertHistorical>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alerts = MachineHeartbeatAlertHistorical::get_by_organization_id(
        &state.pool,
        account.organization_id,
        params.limit,
        params.offset,
    )
    .await?;
    Ok(Json(alerts))
}

/// Acknowledge a machine heartbeat alert and move it to the historical table - used for deprecated
/// machines
#[utoipa::path(
    post,
    path = "/alerts/heartbeat/machine/acknowledge",
    params(MachineAlertParams),
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn acknowledge_machine_alert(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<MachineAlertParams>,
) -> Result<Json<()>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let machine_id = params.machine_id;
    let alert = MachineHeartbeatAlert::get(&state.pool, machine_id)
        .await?
        .ok_or(BackendError::MachineHeartbeatAlertNotFound(machine_id))?;
    if alert.organization_id != account.organization_id {
        return Err(BackendError::MachineHeartbeatAlertNotFound(machine_id));
    }
    MachineHeartbeatAlert::resolve(&state.pool, params.machine_id).await?;
    Ok(Json(()))
}

/// Get all active node heartbeat alerts
#[utoipa::path(
    get,
    path = "/alerts/heartbeat/node/active",
    responses(
        (status = 200, body = [NodeHeartbeatAlert]),
        (status = 404)
    )
)]
pub async fn node_active_alerts(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<NodeHeartbeatAlert>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alerts =
        NodeHeartbeatAlert::get_by_organization_id(&state.pool, account.organization_id).await?;
    Ok(Json(alerts))
}

/// Get historical node heartbeat alerts
#[utoipa::path(
    get,
    path = "/alerts/heartbeat/node/history",
    params(PaginationParams),
    responses(
        (status = 200, body = [NodeHeartbeatAlertHistorical]),
        (status = 404)
    )
)]
pub async fn node_alert_history(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Vec<NodeHeartbeatAlertHistorical>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alerts = NodeHeartbeatAlertHistorical::get_by_organization_id(
        &state.pool,
        account.organization_id,
        params.limit,
        params.offset,
    )
    .await?;
    Ok(Json(alerts))
}

/// Acknowledge a node heartbeat alert and move it to the historical table - used for deprecated
/// nodes
#[utoipa::path(
    post,
    path = "/alerts/heartbeat/node/acknowledge",
    params(NodeAlertParams),
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn acknowledge_node_alert(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<NodeAlertParams>,
) -> Result<Json<()>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let node_id = params.node_id;
    let alert = NodeHeartbeatAlert::get(&state.pool, node_id.clone())
        .await?
        .ok_or(BackendError::NodeHeartbeatAlertNotFound(node_id.clone()))?;
    if alert.organization_id != account.organization_id {
        return Err(BackendError::NodeHeartbeatAlertNotFound(node_id.clone()));
    }
    NodeHeartbeatAlert::resolve(&state.pool, node_id.clone()).await?;
    Ok(Json(()))
}
