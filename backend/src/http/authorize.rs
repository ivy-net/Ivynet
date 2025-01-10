use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use base64::Engine as _;
use ethers::types::Address;
use sendgrid::v3::{Email, Message, Personalization};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::BackendError;
use db::{
    machine::Machine,
    verification::{Verification, VerificationType},
    Account, Client,
};

use super::HttpState;

#[derive(Deserialize, Debug, Clone, ToSchema)]
pub struct AuthorizationCredentials {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Debug, Clone, ToSchema)]
pub struct ForgotPasswordCredentials {
    pub email: String,
}

#[derive(Deserialize, Debug, Clone, ToSchema)]
pub struct SetPasswordCredentials {
    pub verification_id: Uuid,
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

            Ok((
                jar.add({
                    let mut session_cookie = Cookie::new("session_id", uuid.clone());
                    session_cookie.set_secure(true);
                    session_cookie.set_same_site(SameSite::None);
                    session_cookie
                }),
                AuthorizationResponse { uuid }.into(),
            ))
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
    path = "/authorize/forgot_password",
    request_body = ForgotPasswordCredentials,
    responses(
        (status = 200, body = bool),
        (status = 404)
    )
)]
pub async fn forgot_password(
    State(state): State<HttpState>,
    Json(credentials): Json<ForgotPasswordCredentials>,
) -> Result<Json<bool>, BackendError> {
    let verification = Account::set_verification(&state.pool, &credentials.email).await?;

    if let (Some(sender), Some(sender_address), Some(pass_reset_template)) =
        (state.sender, state.sender_email, state.pass_reset_template)
    {
        let mut arguments = HashMap::with_capacity(1);
        //TODO: Setting this url has to be properly set
        arguments.insert(
            "confirmation_url".to_string(),
            format!("{}password_reset/{}", state.root_url, verification.verification_id),
        );

        sender
            .send(
                &Message::new(Email::new(&sender_address))
                    .set_template_id(&pass_reset_template)
                    .add_personalization(
                        Personalization::new(Email::new(credentials.email))
                            .add_dynamic_template_data(arguments),
                    ),
            )
            .await?;
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
    Json(credentials): Json<SetPasswordCredentials>,
) -> Result<Json<bool>, BackendError> {
    let verification = Verification::get(&state.pool, credentials.verification_id).await?;
    if verification.verification_type != VerificationType::User {
        return Err(BackendError::BadId);
    }
    let account = Account::get(&state.pool, verification.associated_id as u64).await?;

    account.set_password(&state.pool, &credentials.password).await?;

    verification.delete(&state.pool).await?;
    Ok(true.into())
}

pub async fn verify(
    pool: &PgPool,
    headers: &HeaderMap,
    cache: &memcache::Client,
    jar: &CookieJar,
) -> Result<Account, BackendError> {
    if let Some(auth_header) = headers.get("Authorization") {
        let split = auth_header.to_str().map_err(|_| BackendError::BadCredentials)?.split_once(' ');
        match split {
            Some(("Basic", contents)) => {
                let (username, password) = decode(contents)?;
                if let Some(pass) = password {
                    Ok(Account::verify(pool, &username, &pass).await?)
                } else {
                    Err(BackendError::Unauthorized)
                }
            }
            _ => Err(BackendError::Unauthorized),
        }
    } else {
        let session = jar.get("session_id").ok_or(BackendError::Unauthorized)?.value();

        let user_id = cache.get(session)?.ok_or(BackendError::Unauthorized)?;
        cache.set(session, user_id, 15 * 60)?;
        Ok(Account::get(pool, user_id).await?)
    }
}

fn decode(input: &str) -> Result<(String, Option<String>), BackendError> {
    // Decode from base64 into a string
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(input)
        .map_err(|_| BackendError::BadCredentials)?;
    let decoded = String::from_utf8(decoded).map_err(|_| BackendError::BadCredentials)?;

    // Return depending on if password is present
    Ok(if let Some((id, password)) = decoded.split_once(':') {
        (id.to_string(), Some(password.to_string()))
    } else {
        (decoded.to_string(), None)
    })
}

pub async fn verify_machine_ownership(
    account: &Account,
    State(state): State<HttpState>,
    machine_id: String,
) -> Result<Machine, BackendError> {
    let machine_id = machine_id.parse::<Uuid>().map_err(|_| BackendError::BadId)?;
    let machine = Machine::get(&state.pool, machine_id).await?.ok_or(BackendError::BadId)?;
    if account.role.can_write() &&
        !account
            .all_machines(&state.pool)
            .await?
            .into_iter()
            .filter_map(|m| if m.machine_id == machine.machine_id { Some(m) } else { None })
            .collect::<Vec<_>>()
            .is_empty()
    {
        Ok(machine)
    } else {
        Err(BackendError::Unauthorized)
    }
}

pub async fn verify_client_ownership(
    account: &Account,
    pool: &PgPool,
    client_id: &str,
) -> Result<Client, BackendError> {
    let id = client_id.parse::<Address>().map_err(|_| BackendError::BadId)?;

    let clients = account.clients(pool).await?;
    let result = clients.iter().find(|c| c.client_id == id);

    match result {
        Some(client) => Ok(client.clone()),
        None => Err(BackendError::Unauthorized),
    }
}
