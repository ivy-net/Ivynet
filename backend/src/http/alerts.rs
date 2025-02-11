use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, NaiveDateTime};
use db::alerts::{
    alert_actor::AlertType, alerts_active::ActiveAlert, alerts_historical::HistoryAlert,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::{BackendAlertError, BackendError},
    http::authorize,
};

use super::HttpState;

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
pub struct HistoricalAlertParams {
    pub from: i64,
    pub to: i64,
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
pub struct AcknowledgeAlertParams {
    pub alert_id: u64,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct AlertReport {
    pub alert_id: i64,
    pub alert_type: AlertType,
    pub machine_id: Uuid,
    pub node_name: String,
    pub created_at: NaiveDateTime,
    pub acknowledged_at: Option<NaiveDateTime>,
}

impl From<ActiveAlert> for AlertReport {
    fn from(alert: ActiveAlert) -> Self {
        Self {
            alert_id: alert.alert_id,
            alert_type: alert.alert_type,
            machine_id: alert.machine_id,
            node_name: alert.node_name,
            created_at: alert.created_at,
            acknowledged_at: alert.acknowledged_at,
        }
    }
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct HistoricalAlertReport {
    pub alert_id: i64,
    pub alert_type: AlertType,
    pub machine_id: Uuid,
    pub node_name: String,
    pub created_at: NaiveDateTime,
    pub acknowledged_at: Option<NaiveDateTime>,
    pub resolved_at: NaiveDateTime,
}

impl From<HistoryAlert> for HistoricalAlertReport {
    fn from(alert: HistoryAlert) -> Self {
        Self {
            alert_id: alert.alert_id,
            alert_type: alert.alert_type,
            machine_id: alert.machine_id,
            node_name: alert.node_name,
            created_at: alert.created_at,
            acknowledged_at: alert.acknowledged_at,
            resolved_at: alert.resolved_at,
        }
    }
}

pub async fn active_alerts(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<AlertReport>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alerts = ActiveAlert::all_alerts_by_org(&state.pool, account.organization_id)
        .await?
        .into_iter()
        .map(AlertReport::from)
        .collect();
    Ok(Json(alerts))
}

pub async fn acknowledge_alert(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<AcknowledgeAlertParams>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alert_id = params.alert_id;
    let alert = ActiveAlert::get(&state.pool, alert_id as i64)
        .await?
        .ok_or(BackendAlertError::AlertNotFound(alert_id))?;
    if alert.organization_id != account.organization_id {
        return Err(BackendAlertError::AlertNotFound(alert_id).into());
    }
    ActiveAlert::acknowledge(&state.pool, params.alert_id as i64).await?;
    Ok(())
}

pub async fn alert_history(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<HistoricalAlertParams>,
) -> Result<Json<Vec<HistoricalAlertReport>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let from = DateTime::from_timestamp(params.from, 0)
        .ok_or(BackendError::MalformedParameter("from".to_string(), params.from.to_string()))?
        .naive_utc();
    let to = DateTime::from_timestamp(params.to, 0)
        .ok_or(BackendError::MalformedParameter("to".to_string(), params.to.to_string()))?
        .naive_utc();
    let alerts: Vec<HistoricalAlertReport> =
        HistoryAlert::alerts_by_org_between(&state.pool, account.organization_id, from, to)
            .await?
            .into_iter()
            .map(HistoricalAlertReport::from)
            .collect();
    Ok(Json(alerts))
}
