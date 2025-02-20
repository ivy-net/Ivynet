use crate::error::BackendError;
use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use ivynet_database::operator_keys::OperatorKey;
use ethers::types::Address;
use std::collections::HashMap;

use super::{authorize, HttpState};

/// Get all operator keys for the organization
#[utoipa::path(
    get,
    path = "/pubkey",
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
    path = "/pubkey",
    params(
        ("public_key" = String, Query, description = "The public key to create"),
        ("name" = String, Query, description = "The name for the key"),
    ),
    responses(
        (status = 200),
        (status = 400)
    )
)]
pub async fn create_key(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<HashMap<String, String>>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let public_key = params.get("public_key").ok_or(BackendError::MalformedParameter(
        "public_key".to_string(),
        "Public key cannot be empty".to_string(),
    ))?;

    let name = params.get("name").ok_or(BackendError::MalformedParameter(
        "name".to_string(),
        "Name cannot be empty".to_string(),
    ))?;

    let public_key: Address = public_key.parse().map_err(|_| {
        BackendError::MalformedParameter(
            "public_key".to_string(),
            "Invalid public key format".to_string(),
        )
    })?;

    OperatorKey::create(&state.pool, account.organization_id, name, &public_key).await?;
    Ok(())
}

/// Update an operator key's name
#[utoipa::path(
    put,
    path = "/pubkey",
    params(
        ("public_key" = String, Query, description = "The public key to update"),
        ("name" = String, Query, description = "The new name for the key"),
    ),
    responses(
        (status = 200),
        (status = 400)
    )
)]
pub async fn update_key_name(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<HashMap<String, String>>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let public_key = params.get("public_key").ok_or(BackendError::MalformedParameter(
        "public_key".to_string(),
        "Public key cannot be empty".to_string(),
    ))?;

    println!("public_key: {}", public_key);

    let name = params.get("name").ok_or(BackendError::MalformedParameter(
        "name".to_string(),
        "Name cannot be empty".to_string(),
    ))?;

    println!("name: {}", name);

    let public_key: Address = public_key.parse().map_err(|_| {
        BackendError::MalformedParameter(
            "public_key".to_string(),
            "Invalid public key format".to_string(),
        )
    })?;

    OperatorKey::change_name(&state.pool, account.organization_id, &public_key, name).await?;
    Ok(())
}

/// Delete an operator key
#[utoipa::path(
    delete,
    path = "/pubkey",
    params(
        ("public_key" = String, Query, description = "The public key to delete"),
    ),
    responses(
        (status = 200),
        (status = 400)
    )
)]
pub async fn delete_key(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Query(params): Query<HashMap<String, String>>,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let public_key = params.get("public_key").ok_or(BackendError::MalformedParameter(
        "public_key".to_string(),
        "Public key cannot be empty".to_string(),
    ))?;

    let public_key: Address = public_key.parse().map_err(|_| {
        BackendError::MalformedParameter(
            "public_key".to_string(),
            "Invalid public key format".to_string(),
        )
    })?;

    OperatorKey::delete(&state.pool, account.organization_id, &public_key).await?;
    Ok(())
}
