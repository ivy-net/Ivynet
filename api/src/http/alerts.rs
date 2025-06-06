use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use chrono::DateTime;
use ivynet_alerts::{AlertFlags, AlertType};
use ivynet_database::{
    alerts::{
        node::{alerts_active::NodeActiveAlert, alerts_historical::NodeHistoryAlert},
        org::{
            alerts_active::OrganizationActiveAlert, alerts_historical::OrganizationHistoryAlert,
        },
    },
    service_settings::ServiceType,
    NotificationSettings, ServiceSettings,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{error::BackendError, http::authorize};

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

/* -------------------------------------
----BASE NODE ALERT FUNCTIONALITY-------
------------------------------------- */

/// Get all active alerts for every machine
#[utoipa::path(
    get,
    path = "/alerts/node/active",
    responses(
        (status = 200, body = [NodeActiveAlert]),
        (status = 404)
    )
)]
pub async fn node_active_alerts(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<NodeActiveAlert>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alerts = NodeActiveAlert::all_alerts_by_org(&state.pool, account.organization_id).await?;
    Ok(Json(alerts))
}

/// Remove an active alert - moves to historical
#[utoipa::path(
    post,
    path = "/alerts/node/remove",
    params(AcknowledgeAlertParams),
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn node_remove_alert(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<AcknowledgeAlertParams>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alert_id = params.alert_id;
    let alert = NodeActiveAlert::get(&state.pool, alert_id)
        .await?
        .ok_or(BackendError::AlertNotFound(alert_id))?;
    if alert.organization_id != account.organization_id {
        return Err(BackendError::AlertNotFound(alert_id));
    }
    NodeActiveAlert::resolve_alert(&state.pool, alert_id).await?;
    Ok(())
}

/// Acknowledge an alert
#[utoipa::path(
    post,
    path = "/alerts/node/acknowledge",
    params(AcknowledgeAlertParams),
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn node_acknowledge_alert(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<AcknowledgeAlertParams>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alert_id = params.alert_id;
    let alert = NodeActiveAlert::get(&state.pool, alert_id)
        .await?
        .ok_or(BackendError::AlertNotFound(alert_id))?;
    if alert.organization_id != account.organization_id {
        return Err(BackendError::AlertNotFound(alert_id));
    }
    NodeActiveAlert::acknowledge(&state.pool, params.alert_id).await?;
    Ok(())
}

/// Get historical alerts for all machines
#[utoipa::path(
    get,
    path = "/alerts/node/history",
    params(HistoricalAlertParams),
    responses(
        (status = 200, body = [NodeHistoryAlert]),
        (status = 404)
    )
)]
pub async fn node_alert_history(
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
            .await?;
    Ok(Json(alerts))
}

/* --------------------------------------
--BASE ORGANIZATION ALERT FUNCTIONALITY--
----------------------------------------- */

/// Get all active alerts for your organization
#[utoipa::path(
    get,
    path = "/alerts/org/active",
    responses(
        (status = 200, body = [OrganizationActiveAlert]),
        (status = 404)
    )
)]
pub async fn org_active_alerts(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<OrganizationActiveAlert>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alerts = OrganizationActiveAlert::all_alerts_by_org(&state.pool, account.organization_id)
        .await?
        .into_iter()
        .collect();
    Ok(Json(alerts))
}

/// Acknowledge an organization alert - equivalent to resolution of the alert
#[utoipa::path(
    post,
    path = "/alerts/org/acknowledge",
    params(AcknowledgeAlertParams),
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn org_acknowledge_alert(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<AcknowledgeAlertParams>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let alert_id = params.alert_id;
    let alert = OrganizationActiveAlert::get(&state.pool, alert_id, account.organization_id)
        .await?
        .ok_or(BackendError::AlertNotFound(alert_id))?;
    if alert.organization_id != account.organization_id {
        return Err(BackendError::AlertNotFound(alert_id));
    }
    OrganizationActiveAlert::resolve_alert(&state.pool, params.alert_id, account.organization_id)
        .await?;
    Ok(())
}

/// Get historical alerts for your organization
#[utoipa::path(
    get,
    path = "/alerts/org/history",
    params(HistoricalAlertParams),
    responses(
        (status = 200, body = [OrganizationHistoryAlert]),
        (status = 404)
    )
)]
pub async fn org_alert_history(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<HistoricalAlertParams>,
) -> Result<Json<Vec<OrganizationHistoryAlert>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let from = DateTime::from_timestamp(params.from, 0)
        .ok_or(BackendError::MalformedParameter("from".to_string(), params.from.to_string()))?
        .naive_utc();
    let to = DateTime::from_timestamp(params.to, 0)
        .ok_or(BackendError::MalformedParameter("to".to_string(), params.to.to_string()))?
        .naive_utc();
    let alerts: Vec<OrganizationHistoryAlert> = OrganizationHistoryAlert::alerts_by_org_between(
        &state.pool,
        account.organization_id,
        from,
        to,
    )
    .await?
    .into_iter()
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
    pub integration_keys: Vec<String>,
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
        let mut integration_keys = Vec::new();

        for setting in value.1 {
            match setting.settings_type {
                ServiceType::Email => emails.push(setting.settings_value.clone()),
                ServiceType::Telegram => chats.push(setting.settings_value.clone()),
                ServiceType::PagerDuty => integration_keys.push(setting.settings_value.clone()),
            }
        }

        Self {
            email: EmailSettings { enabled: value.0.email, emails },
            telegram: TelegramSettings { enabled: value.0.telegram, chats },
            pagerduty: PagerDutySettings { enabled: value.0.pagerduty, integration_keys },
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

/// Set notification service settings - email, telegram, pagerduty - and information for each
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

    // Handle email settings
    ServiceSettings::delete_by_org_and_type(
        &state.pool,
        account.organization_id as u64,
        ServiceType::Email,
    )
    .await?;

    NotificationSettings::add_emails(
        &state.pool,
        account.organization_id as u64,
        &settings.email.emails,
    )
    .await?;

    // Handle PagerDuty settings
    ServiceSettings::delete_by_org_and_type(
        &state.pool,
        account.organization_id as u64,
        ServiceType::PagerDuty,
    )
    .await?;
    if !settings.pagerduty.integration_keys.is_empty() {
        NotificationSettings::add_pagerduty_keys(
            &state.pool,
            account.organization_id as u64,
            &settings.pagerduty.integration_keys,
        )
        .await?;
    }

    // Handle Telegram settings
    ServiceSettings::delete_by_org_and_type(
        &state.pool,
        account.organization_id as u64,
        ServiceType::Telegram,
    )
    .await?;

    if !settings.telegram.chats.is_empty() {
        NotificationSettings::add_many_chats(
            &state.pool,
            account.organization_id as u64,
            &settings.telegram.chats,
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

/// Get human-readable enabled notification flags
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
    post,
    path = "/alerts/notifications/set_flag",
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

/// Update multiple notification flags
#[utoipa::path(
    post,
    path = "/alerts/notifications/set_flags",
    request_body = Vec<AlertFlagUpdate>,
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn update_multiple_alert_flags(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(payload): Json<Vec<AlertFlagUpdate>>,
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

    // Update each flag based on the payload.
    for update in payload {
        flags.set_alert_to(&update.alert, update.enabled)?;
    }

    // Save the updated flags.
    NotificationSettings::set_alert_flags(
        &state.pool,
        account.organization_id as u64,
        flags.into(),
    )
    .await?;

    Ok(())
}
