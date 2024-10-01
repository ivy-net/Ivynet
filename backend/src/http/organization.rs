use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use ivynet_core::ethers::types::Address;
use sendgrid::v3::{Email, Message, Personalization};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    db::{
        metric::Metric,
        node::DbNode,
        verification::{Verification, VerificationType},
        Account, Node, Organization, Role,
    },
    error::BackendError,
};

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
            format!("{}/organization_confirm/{}", state.root_url, verification.verification_id),
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
    Ok(Organization::get(&state.pool, id).await?.into())
}

#[utoipa::path(
    get,
    path = "/organization/nodes",
    responses(
        (status = 200, body = [Node]),
        (status = 404)
    )
)]
pub async fn nodes(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<Node>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    Ok(DbNode::get_all_for_account(&state.pool, &account).await?.into())
}

#[utoipa::path(
    get,
    path = "/organization/nodes/:id/metrics",
    responses(
        (status = 200, body = [Metric]),
        (status = 404)
    )
)]
pub async fn metrics(
    headers: HeaderMap,
    State(state): State<HttpState>,
    Path(id): Path<String>,
    jar: CookieJar,
) -> Result<Json<Vec<Metric>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let node_id = id.parse::<Address>().map_err(|_| BackendError::InvalidNodeId)?;

    let account_nodes = DbNode::get_all_for_account(&state.pool, &account).await?;

    let node = {
        let mut ret = None;
        for node in account_nodes {
            if node.node_id == node_id {
                ret = Some(node);
                break;
            }
        }
        ret
    };
    if let Some(node) = node {
        Ok(Metric::get_all_for_node(&state.pool, &node).await?.into())
    } else {
        Err(BackendError::InvalidNodeId)
    }
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
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    if !account.role.is_admin() {
        return Err(BackendError::InsufficientPriviledges);
    }

    let org = Organization::get(&state.pool, account.organization_id as u64).await?;
    let new_acc = org.invite(&state.pool, &request.email, request.role).await?;

    if let (Some(sender), Some(sender_address), Some(inv_template)) =
        (state.sender, state.sender_email, state.user_verification_template)
    {
        let mut arguments = HashMap::with_capacity(1);
        arguments.insert("organization_name".to_string(), org.name);
        //TODO: Setting this url has to be properly set
        arguments.insert(
            "confirmation_url".to_string(),
            format!("{}/password_set/{}", state.root_url, new_acc.verification_id),
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
