use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::DateTime;
use ivynet_database::{
    alerts::{alerts_active::ActiveAlert, alerts_historical::HistoryAlert},
    notification_settings::SettingsType,
    Notifications, NotificationsSettings,
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

#[derive(Debug, Clone, Copy, ToSchema, Deserialize)]
pub struct AcknowledgeAlertParams {
    pub alert_id: Uuid,
}

/* --------------------------------
----BASE ALERT FUNCTIONALITY-------
----------------------------------- */

/// Get all active alerts for every machine
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
        .map(ActiveAlert::from)
        .collect();
    Ok(Json(alerts))
}

/// Acknowledge an alert
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

/// Get historical alerts for all machines
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
            .map(HistoryAlert::from)
            .collect();
    Ok(Json(alerts))
}

/* ---------------------------------------
-------SERVICE FLAG FUNCTIONALITY---------
------------------------------------------ */

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct NotificationServiceSettings {
    pub telegram: TelegramSettings,
    pub email: EmailSettings,
    pub pagerduty: PagerDutySettings,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct TelegramSettings {
    pub enabled: bool,
    pub chats: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct PagerDutySettings {
    pub enabled: bool,
    pub integration_key: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct EmailSettings {
    pub enabled: bool,
    pub emails: Vec<String>,
}

impl From<(Notifications, Vec<NotificationsSettings>)> for NotificationServiceSettings {
    fn from(value: (Notifications, Vec<NotificationsSettings>)) -> Self {
        let mut emails = Vec::new();
        let mut chats = Vec::new();
        let mut integration_key = None;

        for setting in value.1 {
            match setting.settings_type {
                SettingsType::Email => emails.push(setting.settings_value.clone()),
                SettingsType::Telegram => chats.push(setting.settings_value.clone()),
                SettingsType::PagerDuty => integration_key = Some(setting.settings_value.clone()),
            }
        }

        Self {
            email: EmailSettings { enabled: value.0.email, emails },
            telegram: TelegramSettings { enabled: value.0.telegram, chats },
            pagerduty: PagerDutySettings { enabled: value.0.pagerduty, integration_key },
        }
    }
}

/// Listing current service settings for notifications - Email, Telegram, PagerDuty
#[utoipa::path(
    get,
    path = "/alerts/notifications/services",
    responses(
        (status = 200, body = NotificationSettings),
        (status = 404)
    )
)]
pub async fn get_notification_settings(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<NotificationsSettings>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let notifications =
        Notifications::get(&state.pool, account.organization_id as u64).await.unwrap_or_default();
    let not_settings =
        Notifications::get_notification_settings(&state.pool, account.organization_id as u64, None)
            .await?;

    let response: NotificationsSettings = (notifications, not_settings).into();

    Ok(response.into())
}

/* ---------------------------------------
----NOTIFICATION FLAG FUNCTIONALITY-------
------------------------------------------ */
