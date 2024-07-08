use axum::{
    extract::{Path, State},
    Json,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    db::{
        verification::{Verification, VerificationType},
        Account,
    },
    error::BackendError,
};

use super::HttpState;

#[derive(Deserialize, Debug, Clone, ToSchema)]
pub struct AuthorizationCredentials {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Debug, Clone, ToSchema)]
pub struct SetPasswordCredentials {
    pub password: String,
}

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct AuthorizationResponse {
    pub uuid: String,
}

#[utoipa::path(
    post,
    path = "/authorize", 
    request_body = AuthorizationCredentials,
    responses(
        (status = 200, body = AuthorizationResponse),
        (status = 404)
    )
)]
pub async fn authorize(
    State(state): State<HttpState>,
    jar: CookieJar,
    Json(credentials): Json<AuthorizationCredentials>,
) -> Result<(CookieJar, Json<AuthorizationResponse>), BackendError> {
    match Account::verify(&state.pool, &credentials.email, &credentials.password).await {
        Ok(account) => {
            let uuid = Uuid::new_v4().to_string();
            state.cache.set(&uuid, account.user_id, 15 * 60)?;

            Ok((jar.add(Cookie::new("session_id", uuid.clone())), AuthorizationResponse { uuid }.into()))
        }
        Err(_) => Err(BackendError::BadCredentials),
    }
}

#[utoipa::path(
    get,
    path = "/authorize/invitation/{id}",
    params(
        ("id", description = "Invitation id.")
    ),
    responses(
        (status = 200, body = bool),
        (status = 404)
    )
)]
pub async fn check_invitation(
    State(state): State<HttpState>,
    Path(id): Path<Uuid>,
) -> Result<Json<bool>, BackendError> {
    let verification = Verification::get(&state.pool, id).await?;
    if verification.verification_type != VerificationType::User {
        return Err(BackendError::BadId);
    }
    Ok(true.into())
}

#[utoipa::path(
    post,
    path = "/authorize/set_password",
    request_body = SetPasswordCredentials,
    responses(
        (status = 200, body = bool),
        (status = 404)
    )
)]
pub async fn set_password(
    State(state): State<HttpState>,
    Path(id): Path<Uuid>,
    Json(credentials): Json<SetPasswordCredentials>,
) -> Result<Json<bool>, BackendError> {
    let verification = Verification::get(&state.pool, id).await?;
    if verification.verification_type != VerificationType::User {
        return Err(BackendError::BadId);
    }
    let account = Account::get(&state.pool, verification.associated_id as u64).await?;
    if !account.password.is_empty() {
        return Err(BackendError::AlreadySet);
    }

    account.set_password(&state.pool, &credentials.password).await?;

    verification.delete(&state.pool).await?;
    Ok(true.into())
}

pub async fn verify(pool: &PgPool, cache: &memcache::Client, jar: &CookieJar) -> Result<Account, BackendError> {
    let session = jar.get("session_id").ok_or(BackendError::Unauthorized)?.value();

    let user_id = cache.get(session)?.ok_or(BackendError::Unauthorized)?;
    Account::get(pool, user_id).await
}
