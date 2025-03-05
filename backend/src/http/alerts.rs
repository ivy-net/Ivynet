use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::DateTime;
use db::alerts::{alerts_active::ActiveAlert, alerts_historical::HistoryAlert};
use serde::Deserialize;
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

#[derive(Debug, Clone, Copy, ToSchema, Deserialize)]
pub struct AcknowledgeAlertParams {
    pub alert_id: Uuid,
}

#[utoipa::path(
    get,
    path = "/alerts/active",
    responses(
        (status = 200, body = [ActiveAlert]),
        (status = 404)
    )
)]
pub async fn active_alerts(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<ActiveAlert>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alerts = ActiveAlert::all_alerts_by_org(&state.pool, account.organization_id)
        .await?
        .into_iter()
        .collect();
    Ok(Json(alerts))
}

#[utoipa::path(
    post,
    path = "/alerts/acknowledge",
    request_body = AcknowledgeAlertParams,
    responses(
        (status = 200, body = ()),
        (status = 404)
    )
)]
pub async fn acknowledge_alert(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<AcknowledgeAlertParams>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alert_id = params.alert_id;
    let alert = ActiveAlert::get(&state.pool, alert_id)
        .await?
        .ok_or(BackendAlertError::AlertNotFound(alert_id))?;
    if alert.organization_id != account.organization_id {
        return Err(BackendAlertError::AlertNotFound(alert_id).into());
    }
    ActiveAlert::acknowledge(&state.pool, params.alert_id).await?;
    Ok(())
}

#[utoipa::path(
    get,
    path = "/alerts/history",
    request_body = HistoricalAlertParams,
    responses(
        (status = 200, body = [HistoryAlert]),
        (status = 404)
    )
)]
pub async fn alert_history(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<HistoricalAlertParams>,
) -> Result<Json<Vec<HistoryAlert>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let from = DateTime::from_timestamp(params.from, 0)
        .ok_or(BackendError::MalformedParameter("from".to_string(), params.from.to_string()))?
        .naive_utc();
    let to = DateTime::from_timestamp(params.to, 0)
        .ok_or(BackendError::MalformedParameter("to".to_string(), params.to.to_string()))?
        .naive_utc();
    let alerts: Vec<HistoryAlert> =
        HistoryAlert::alerts_by_org_between(&state.pool, account.organization_id, from, to)
            .await?
            .into_iter()
            .collect();
    Ok(Json(alerts))
}
