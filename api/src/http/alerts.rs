use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::DateTime;
use ivynet_alerts::{AlertFlags, AlertType};
use ivynet_database::{
    alerts::{node_alerts_active::NodeActiveAlert, node_alerts_historical::NodeHistoryAlert},
    notification_settings::ServiceType,
    NotificationSettings, ServiceSettings,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::{BackendAlertError, BackendError},
    http::authorize,
};

use super::HttpState;

#[derive(Debug, Clone, Copy, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct HistoricalAlertParams {
    pub from: i64,
    pub to: i64,
}

#[derive(Debug, Clone, Copy, ToSchema, Deserialize, utoipa::IntoParams)]
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
        (status = 200, body = [NodeActiveAlert]),
        (status = 404)
    )
)]
pub async fn active_alerts(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<NodeActiveAlert>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alerts = NodeActiveAlert::all_alerts_by_org(&state.pool, account.organization_id)
        .await?
        .into_iter()
        .map(NodeActiveAlert::from)
        .collect();
    Ok(Json(alerts))
}

/// Acknowledge an alert
#[utoipa::path(
    post,
    path = "/alerts/acknowledge",
    params(AcknowledgeAlertParams),
    responses(
        (status = 200),
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
    let alert = NodeActiveAlert::get(&state.pool, alert_id)
        .await?
        .ok_or(BackendAlertError::AlertNotFound(alert_id))?;
    if alert.organization_id != account.organization_id {
        return Err(BackendAlertError::AlertNotFound(alert_id).into());
    }
    NodeActiveAlert::acknowledge(&state.pool, params.alert_id).await?;
    Ok(())
}

/// Get historical alerts for all machines
#[utoipa::path(
    get,
    path = "/alerts/history",
    params(HistoricalAlertParams),
    responses(
        (status = 200, body = [NodeHistoryAlert]),
        (status = 404)
    )
)]
pub async fn alert_history(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<HistoricalAlertParams>,
) -> Result<Json<Vec<NodeHistoryAlert>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let from = DateTime::from_timestamp(params.from, 0)
        .ok_or(BackendError::MalformedParameter("from".to_string(), params.from.to_string()))?
        .naive_utc();
    let to = DateTime::from_timestamp(params.to, 0)
        .ok_or(BackendError::MalformedParameter("to".to_string(), params.to.to_string()))?
        .naive_utc();
    let alerts: Vec<NodeHistoryAlert> =
        NodeHistoryAlert::alerts_by_org_between(&state.pool, account.organization_id, from, to)
            .await?
            .into_iter()
            .map(NodeHistoryAlert::from)
            .collect();
    Ok(Json(alerts))
}

/* ---------------------------------------
-------SERVICE FLAG FUNCTIONALITY---------
------------------------------------------ */

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

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct NotificationServiceSettings {
    pub telegram: TelegramSettings,
    pub email: EmailSettings,
    pub pagerduty: PagerDutySettings,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct NotificationServiceFlags {
    telegram: bool,
    email: bool,
    pagerduty: bool,
}

impl From<(NotificationSettings, Vec<ServiceSettings>)> for NotificationServiceSettings {
    fn from(value: (NotificationSettings, Vec<ServiceSettings>)) -> Self {
        let mut emails = Vec::new();
        let mut chats = Vec::new();
        let mut integration_key = None;

        for setting in value.1 {
            match setting.settings_type {
                ServiceType::Email => emails.push(setting.settings_value.clone()),
                ServiceType::Telegram => chats.push(setting.settings_value.clone()),
                ServiceType::PagerDuty => integration_key = Some(setting.settings_value.clone()),
            }
        }

        Self {
            email: EmailSettings { enabled: value.0.email, emails },
            telegram: TelegramSettings { enabled: value.0.telegram, chats },
            pagerduty: PagerDutySettings { enabled: value.0.pagerduty, integration_key },
        }
    }
}

/// Listing current notification service settings for organization
#[utoipa::path(
    get,
    path = "/alerts/services",
    responses(
        (status = 200, body = NotificationServiceSettings),
        (status = 404)
    )
)]
pub async fn get_notification_service_settings(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<NotificationServiceSettings>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let notifications = NotificationSettings::get(&state.pool, account.organization_id as u64)
        .await
        .unwrap_or_default();
    let not_settings = NotificationSettings::get_service_settings(
        &state.pool,
        account.organization_id as u64,
        None,
    )
    .await?;

    let response: NotificationServiceSettings = (notifications, not_settings).into();

    Ok(response.into())
}

/// Set new notification service settings - email, telegram, pagerduty - and information for each
#[utoipa::path(
    post,
    path = "/alerts/services",
    request_body = NotificationServiceSettings,
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn set_notification_service_settings(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(settings): Json<NotificationServiceSettings>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    if !account.role.can_write() {
        return Err(BackendError::InsufficientPriviledges);
    }

    NotificationSettings::set(
        &state.pool,
        account.organization_id as u64,
        settings.email.enabled,
        settings.telegram.enabled,
        settings.pagerduty.enabled,
    )
    .await?;
    NotificationSettings::set_emails(
        &state.pool,
        account.organization_id as u64,
        &settings.email.emails,
    )
    .await?;

    if let Some(ref integration_key) = settings.pagerduty.integration_key {
        NotificationSettings::set_pagerduty_integration(
            &state.pool,
            account.organization_id as u64,
            integration_key,
        )
        .await?;
    }

    Ok(())
}

/// Turn notification services on or off
#[utoipa::path(
    post,
    path = "/alerts/services/set_flags",
    request_body = NotificationServiceFlags,
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn set_notification_service_flags(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(flags): Json<NotificationServiceFlags>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    NotificationSettings::set(
        &state.pool,
        account.organization_id as u64,
        flags.email,
        flags.telegram,
        flags.pagerduty,
    )
    .await?;

    Ok(())
}

/* ---------------------------------------
----NOTIFICATION FLAG FUNCTIONALITY-------
------------------------------------------ */

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct AlertFlagUpdate {
    pub alert: AlertType,
    pub enabled: bool,
}

/// List all notification bitflags that can be enabled/disabled
#[utoipa::path(
    get,
    path = "/alerts/notifications/list",
    responses(
        (status = 200, body = Vec<AlertType>),
        (status = 404)
    )
)]
pub async fn list_alert_flags() -> Result<Json<Vec<AlertType>>, BackendError> {
    Ok(Json(AlertType::list_all()))
}

/// Set notification flags
#[utoipa::path(
    post,
    path = "/alerts/notifications",
    request_body = Vec<u64>,
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn set_alert_flags(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(flags): Json<u64>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    if !account.role.can_write() {
        return Err(BackendError::InsufficientPriviledges);
    }

    NotificationSettings::set_alert_flags(&state.pool, account.organization_id as u64, flags)
        .await?;

    Ok(())
}

/// Get notification flags
#[utoipa::path(
    get,
    path = "/alerts/notifications",
    responses(
        (status = 200, body = u64),
        (status = 404)
    )
)]
pub async fn get_alert_flags(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<u64>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let flags =
        NotificationSettings::get_alert_flags(&state.pool, account.organization_id as u64).await?;

    Ok(Json(flags))
}

/// Get human-readable active notification flags
#[utoipa::path(
    get,
    path = "/alerts/notifications/readable",
    responses(
        (status = 200, body = Vec<AlertType>),
        (status = 404)
    )
)]
pub async fn get_alert_flags_human(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<AlertType>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let flags: AlertFlags =
        NotificationSettings::get_alert_flags(&state.pool, account.organization_id as u64)
            .await?
            .into();

    Ok(Json(flags.to_alert_types()))
}

/// Update an individual notification flag
#[utoipa::path(
    patch,
    path = "/alerts/notifications/set_flags",
    request_body = AlertFlagUpdate,
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn update_alert_flag(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(payload): Json<AlertFlagUpdate>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    if !account.role.can_write() {
        return Err(BackendError::InsufficientPriviledges);
    }

    // Retrieve current flags.
    let mut flags: AlertFlags =
        NotificationSettings::get_alert_flags(&state.pool, account.organization_id as u64)
            .await?
            .into();

    let AlertFlagUpdate { alert, enabled } = payload;

    // Update the flag based on the payload.
    flags.set_alert_to(&alert, enabled)?;

    // Save the updated flags.
    NotificationSettings::set_alert_flags(
        &state.pool,
        account.organization_id as u64,
        flags.into(),
    )
    .await?;

    Ok(())
}
