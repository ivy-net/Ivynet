use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use ivynet_heartbeat::alerts::{
    ClientHeartbeatAlert, ClientHeartbeatAlertHistorical, MachineHeartbeatAlert,
    MachineHeartbeatAlertHistorical, NodeHeartbeatAlert, NodeHeartbeatAlertHistorical,
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
