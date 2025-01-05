use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use ivynet_core::grpc::{
    database::{Address, Email as EmailReq, Id, Invite},
    tonic::{IntoRequest, Request},
};
use sendgrid::v3::{Email, Message, Personalization};
use serde::{Deserialize, Serialize};
use tracing::debug;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::BackendError;

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
    if state
        .database
        .exists_account(Request::new(EmailReq { email: request.email.clone() }))
        .await
        .is_ok()
    {
        return Err(BackendError::AccountExists);
    }

    let org = state
        .database
        .new_organization(Request::new(OrganizationCreation {
            name: request.name.clone(),
            admin_email: request.email.clone(),
            admin_password: request.password,
        }))
        .await?
        .into_inner();

    let verification = state
        .database
        .new_verification(Request::new(VerificationCreation {
            verification_type: VerificationType::Organization,
            verification_id: org.id,
        }))
        .await?
        .into_inner();

    let verification_id = Uuid::from_bytes(verification.id);

    if let (Some(sender), Some(sender_address), Some(org_template)) =
        (state.sender, state.sender_email, state.org_verification_template)
    {
        let mut arguments = HashMap::with_capacity(2);
        arguments.insert("organization_name".to_string(), request.name);
        arguments.insert(
            "confirmation_url".to_string(),
            format!("{}organization_confirm/{}", state.root_url, verification_id),
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
    let account = authorize::verify(&state.database, &headers, &state.cache, &jar).await?;

    Ok(state
        .database
        .get_organization(Request::new(Id { id: account.organization_id }))
        .await?
        .into_inner()
        .into())
}

#[utoipa::path(
    get,
    path = "/organization/:id",
    params(
        ("id", description = "Organization id")
    ),
    responses(
        (status = 200, body = Organization),
        (status = 404)
    )
)]
pub async fn get(
    State(state): State<HttpState>,
    Path(id): Path<u64>,
) -> Result<Json<Organization>, BackendError> {
    Ok(state
        .database
        .get_organization(Request::new(Id { id: account.organization_id }))
        .await?
        .into_inner()
        .into())
}

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
    let account = authorize::verify(&state.database, &headers, &state.cache, &jar).await?;

    Ok(state
        .database
        .get_all_machines_for_account(Request::new(Id { id: account.id }))
        .await?
        .into_inner()
        .into())
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

    Ok(state
        .database
        .get_all_avses_for_account(Request::new(Id { id: account.id }))
        .await?
        .into_inner()
        .into())
}

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
    let account = authorize::verify(&state.database, &headers, &state.cache, &jar).await?;
    if authorize::is_admin(&account) {
        return Err(BackendError::InsufficientPriviledges);
    }

    let org = state
        .database
        .get_organization(Request::new(Id { id: account.organization_id }))
        .await?
        .into_inner();
    let account_verification = state
        .database
        .invite_user(Request::new(Invite {
            organization_id: org.id,
            email: request.email.clone(),
            role: request.role.into(),
        }))
        .await?
        .into_inner();

    let verification_id = Uuid::from_slice(account_verification.id);

    if let (Some(sender), Some(sender_address), Some(inv_template)) =
        (state.sender, state.sender_email, state.user_verification_template)
    {
        let mut arguments = HashMap::with_capacity(1);
        arguments.insert("organization_name".to_string(), org.name);
        //TODO: Setting this url has to be properly set
        arguments.insert(
            "confirmation_url".to_string(),
            format!("{}password_set/{}", state.root_url, verification_id),
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

    Ok(InvitationResponse { id: verification_id }.into())
}

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
    let account = authorize::verify(&state.database, &headers, &state.cache, &jar).await?;
    if account.role != Role::Owner {
        return Err(BackendError::InsufficientPriviledges);
    }

    let verification = state
        .database
        .get_verification(Request::new(Address { id: id.to_bytes_le().to_vec() }))
        .await?
        .into_inner();

    if verification.verification_type != VerificationType::Organization {
        return Err(BackendError::BadId);
    }

    let org = state
        .database
        .get_organization(Request::new(Id { id: verification.object_id }))
        .await?
        .into_inner();

    if account.organization_id != org.id {
        return Err(BackendError::InsufficientPriviledges);
    }

    state.database.verify_organization(Request::new(Id { id: org.id })).await?;
    state.database.close_verification(Request::new(Address { id: verification.id })).await?;

    Ok(ConfirmationResponse { success: true }.into())
}
