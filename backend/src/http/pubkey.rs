use axum::{extract::State, http::HeaderMap, Json};
use axum_extra::extract::CookieJar;
use ivynet_core::ethers::types::Address;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{db::operator_keys::OperatorKey, error::BackendError};

use super::{authorize, HttpState};

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateKeyRequest {
    pub name: String,
    pub public_key: Address,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UpdateKeyNameRequest {
    pub name: String,
    pub public_key: Address,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct DeleteKeyRequest {
    pub public_key: Address,
}

/// Get all operator keys for the organization
#[utoipa::path(
    get,
    path = "/keys",
    responses(
        (status = 200, body = [OperatorKey]),
        (status = 404)
    )
)]
pub async fn get_all_keys(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<OperatorKey>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let keys =
        OperatorKey::get_all_keys_for_organization(&state.pool, account.organization_id).await?;
    Ok(Json(keys))
}

/// Create a new operator key
#[utoipa::path(
    post,
    path = "/keys",
    request_body = CreateKeyRequest,
    responses(
        (status = 200),
        (status = 400)
    )
)]
pub async fn create_key(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(request): Json<CreateKeyRequest>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    OperatorKey::create(&state.pool, account.organization_id, &request.name, &request.public_key)
        .await?;
    Ok(())
}

/// Update an operator key's name
#[utoipa::path(
    put,
    path = "/keys",
    request_body = UpdateKeyNameRequest,
    responses(
        (status = 200),
        (status = 400)
    )
)]
pub async fn update_key_name(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(request): Json<UpdateKeyNameRequest>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    OperatorKey::change_name(
        &state.pool,
        account.organization_id,
        &request.public_key,
        &request.name,
    )
    .await?;
    Ok(())
}

/// Delete an operator key
#[utoipa::path(
    delete,
    path = "/keys",
    request_body = DeleteKeyRequest,
    responses(
        (status = 200),
        (status = 400)
    )
)]
pub async fn delete_key(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(request): Json<DeleteKeyRequest>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    OperatorKey::delete(&state.pool, account.organization_id, &request.public_key).await?;
    Ok(())
}
