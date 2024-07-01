use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    Json,
};
use axum_extra::extract::CookieJar;
use sendgrid::v3::{Email, Message, Personalization};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::{
        node::DbNode,
        verification::{Verification, VerificationType},
        Account, Node, Organization, Role,
    },
    error::BackendError,
};

use super::{authorize, HttpState};

#[derive(Serialize, Debug, Clone)]
pub struct CreationResult {
    pub id: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CreationRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct InvitationResponse {
    pub id: Uuid,
}

#[derive(Serialize, Debug, Clone)]
pub struct ConfirmationResponse {
    pub success: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InvitationRequest {
    pub email: String,
    pub role: Role,
}

pub async fn new(
    State(state): State<HttpState>,
    Json(request): Json<CreationRequest>,
) -> Result<Json<CreationResult>, BackendError> {
    if Account::exists(&state.pool, &request.email).await? {
        return Err(BackendError::AccountExists);
    }

    let org = Organization::new(&state.pool, &request.name, false).await?;

    org.attach_admin(&state.pool, &request.email, &request.password)
        .await?;

    let verification = Verification::new(
        &state.pool,
        VerificationType::Organization,
        org.organization_id,
    )
    .await?;

    if let (Some(sender), Some(sender_address), Some(org_template)) = (
        state.sender,
        state.sender_email,
        state.org_verification_template,
    ) {
        let mut arguments = HashMap::with_capacity(2);
        arguments.insert("organization_name".to_string(), request.name);
        arguments.insert(
            "confirmation_url".to_string(),
            format!(
                "{}/organization/confirm/{}",
                state.root_url, verification.verification_id
            ),
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
    Ok(CreationResult {
        id: org.organization_id as u64,
    }
    .into())
}

pub async fn get(
    State(state): State<HttpState>,
    Path(id): Path<u64>,
) -> Result<Json<Organization>, BackendError> {
    Ok(Organization::get(&state.pool, id).await?.into())
}

pub async fn nodes(
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<Node>>, BackendError> {
    let account = authorize::verify(&state.pool, &state.cache, &jar).await?;

    Ok(DbNode::get_all_for_account(&state.pool, &account)
        .await?
        .into())
}

pub async fn invite(
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(request): Json<InvitationRequest>,
) -> Result<Json<InvitationResponse>, BackendError> {
    let account = authorize::verify(&state.pool, &state.cache, &jar).await?;
    if !account.role.is_admin() {
        return Err(BackendError::InsufficientPriviledges);
    }

    let org = Organization::get(&state.pool, account.organization_id as u64).await?;
    let new_acc = org
        .invite(&state.pool, &request.email, request.role)
        .await?;

    Ok(InvitationResponse {
        id: new_acc.verification_id,
    }
    .into())
}

pub async fn confirm(
    State(state): State<HttpState>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<Json<ConfirmationResponse>, BackendError> {
    let account = authorize::verify(&state.pool, &state.cache, &jar).await?;
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
