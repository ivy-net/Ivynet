use std::collections::HashMap;

use crate::error::BackendError;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use db::{
    avs::Avs,
    machine::Machine,
    notifications::SettingsType,
    verification::{Verification, VerificationType},
    Account, Organization, OrganizationNotifications, OrganizationNotificationsSettings, Role,
};
use sendgrid::v3::{Email, Message, Personalization};
use serde::{Deserialize, Serialize};
use tracing::debug;
use utoipa::ToSchema;
use uuid::Uuid;

use super::{authorize, HttpState};

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct CreationResult {
    pub id: u64,
}

#[derive(Deserialize, Debug, Clone, ToSchema)]
pub struct CreationRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct InvitationResponse {
    pub id: Uuid,
}

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct ConfirmationResponse {
    pub success: bool,
}

#[derive(Deserialize, Debug, Clone, ToSchema)]
pub struct InvitationRequest {
    pub email: String,
    pub role: Role,
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

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct NotificationSettings {
    pub telegram: TelegramSettings,
    pub email: EmailSettings,
    pub pagerduty: PagerDutySettings,
}

impl From<(OrganizationNotifications, Vec<OrganizationNotificationsSettings>)>
    for NotificationSettings
{
    fn from(value: (OrganizationNotifications, Vec<OrganizationNotificationsSettings>)) -> Self {
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

/// Create a new organization
#[utoipa::path(
    post,
    path = "/organization",
    request_body = CreationRequest,
    responses(
        (status = 200, body = CreationResult),
        (status = 404)
    )
)]
pub async fn new(
    State(state): State<HttpState>,
    Json(request): Json<CreationRequest>,
) -> Result<Json<CreationResult>, BackendError> {
    if Account::exists(&state.pool, &request.email).await? {
        return Err(BackendError::AccountExists);
    }

    let org = Organization::new(&state.pool, &request.name, false).await?;

    org.attach_admin(&state.pool, &request.email, &request.password).await?;

    let verification =
        Verification::new(&state.pool, VerificationType::Organization, org.organization_id).await?;

    if let (Some(sender), Some(sender_address), Some(org_template)) =
        (state.sender, state.sender_email, state.org_verification_template)
    {
        let mut arguments = HashMap::with_capacity(2);
        arguments.insert("organization_name".to_string(), request.name);
        arguments.insert(
            "confirmation_url".to_string(),
            format!("{}organization_confirm/{}", state.root_url, verification.verification_id),
        );

        sender
            .send(
                &Message::new(Email::new(&sender_address))
                    .set_template_id(&org_template)
                    .add_personalization(
                        Personalization::new(Email::new(request.email))
                            .add_dynamic_template_data(arguments),
                    ),
            )
            .await?;
    }
    Ok(CreationResult { id: org.organization_id as u64 }.into())
}

/// Get your organization
#[utoipa::path(
    get,
    path = "/organization",
    responses(
        (status = 200, body = Organization),
        (status = 404)
    )
)]
pub async fn get_me(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Organization>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    Ok(Organization::get(&state.pool, account.organization_id.try_into().unwrap()).await?.into())
}

//This should not be public
// #[utoipa::path(
//     get,
//     path = "/organization/:id",
//     params(
//         ("id", description = "Organization id")
//     ),
//     responses(
//         (status = 200, body = Organization),
//         (status = 404)
//     )
// )]
// pub async fn get(
//     State(state): State<HttpState>,
//     Path(id): Path<u64>,
// ) -> Result<Json<Organization>, BackendError> {
//     Ok(Organization::get(&state.pool, id).await?.into())
// }

/// Get an overview of all machines in the organization
#[utoipa::path(
    get,
    path = "/organization/machines",
    responses(
        (status = 200, body = [Machine]),
        (status = 404)
    )
)]
pub async fn machines(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<Machine>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    Ok(account.all_machines(&state.pool).await?.into())
}

/// Get an overview of all AVSes in the organization
#[utoipa::path(
    get,
    path = "/organization/avses",
    responses(
        (status = 200, body = [Avs]),
        (status = 404)
    )
)]
pub async fn avses(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<Avs>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    Ok(account.all_avses(&state.pool).await?.into())
}

/// Invite a new user to the organization
#[utoipa::path(
    post,
    path = "/organization/invite",
    request_body = InvitationRequest,
    responses(
        (status = 200, body = InvitationResponse),
        (status = 404)
    )
)]
pub async fn invite(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(request): Json<InvitationRequest>,
) -> Result<Json<InvitationResponse>, BackendError> {
    debug!("Getting account");
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    if !account.role.is_admin() {
        return Err(BackendError::InsufficientPriviledges);
    }

    debug!("Fetching the organization");
    let org = Organization::get(&state.pool, account.organization_id as u64).await?;
    let new_acc = org.invite(&state.pool, &request.email, request.role).await?;
    debug!(
        "Something is missing {:?}, {:?}, {:?}",
        state.sender, state.sender_email, state.user_verification_template
    );
    if let (Some(sender), Some(sender_address), Some(inv_template)) =
        (state.sender, state.sender_email, state.user_verification_template)
    {
        debug!("Sending the email");
        let mut arguments = HashMap::with_capacity(1);
        arguments.insert("organization_name".to_string(), org.name);
        //TODO: Setting this url has to be properly set
        arguments.insert(
            "confirmation_url".to_string(),
            format!("{}password_set/{}", state.root_url, new_acc.verification_id),
        );

        sender
            .send(
                &Message::new(Email::new(&sender_address))
                    .set_template_id(&inv_template)
                    .add_personalization(
                        Personalization::new(Email::new(request.email))
                            .add_dynamic_template_data(arguments),
                    ),
            )
            .await?;
    }

    Ok(InvitationResponse { id: new_acc.verification_id }.into())
}

/// Confirm an invitation to the organization
#[utoipa::path(
    post,
    path = "/organization/confirm/:id",
    params(
        ("id", description = "Verification id for organization")
    ),
    responses(
        (status = 200, body = ConfirmationResponse),
        (status = 404)
    )
)]
pub async fn confirm(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<Json<ConfirmationResponse>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    if account.role != Role::Owner {
        return Err(BackendError::InsufficientPriviledges);
    }

    let verification = Verification::get(&state.pool, id).await?;
    if verification.verification_type != VerificationType::Organization {
        return Err(BackendError::BadId);
    }

    let mut org = Organization::get(&state.pool, verification.associated_id as u64).await?;

    if account.organization_id != org.organization_id {
        return Err(BackendError::InsufficientPriviledges);
    }

    org.verify(&state.pool).await?;
    verification.delete(&state.pool).await?;

    Ok(ConfirmationResponse { success: true }.into())
}

/// Listing current notification settings for organization
#[utoipa::path(
    get,
    path = "/organization/notifications",
    responses(
        (status = 200, body = NotificationSettings),
        (status = 404)
    )
)]
pub async fn get_notification_settings(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<NotificationSettings>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let notifications =
        OrganizationNotifications::get(&state.pool, account.organization_id as u64).await?;
    let not_settings = OrganizationNotifications::get_notification_settings(
        &state.pool,
        account.organization_id as u64,
        None,
    )
    .await?;

    let response: NotificationSettings = (notifications, not_settings).into();

    Ok(response.into())
}

/// Setting new notification settings
#[utoipa::path(
    post,
    path = "/organization/notifications",
    params(
        ("settings", description = "New notification settings to set")
    ),
    responses(
        (status = 200, body = NotificationSettings),
        (status = 404)
    )
)]
pub async fn set_notification_settings(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(settings): Json<NotificationSettings>,
) -> Result<Json<NotificationSettings>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    if !account.role.can_write() {
        return Err(BackendError::InsufficientPriviledges);
    }

    OrganizationNotifications::set(
        &state.pool,
        account.organization_id as u64,
        settings.email.enabled,
        settings.telegram.enabled,
        settings.pagerduty.enabled,
    )
    .await?;
    OrganizationNotifications::set_emails(
        &state.pool,
        account.organization_id as u64,
        &settings.email.emails,
    )
    .await?;

    if let Some(ref integration_key) = settings.pagerduty.integration_key {
        OrganizationNotifications::set_pagerduty_integration(
            &state.pool,
            account.organization_id as u64,
            integration_key,
        )
        .await?;
    }

    Ok(settings.into())
}
