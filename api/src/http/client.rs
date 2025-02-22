use std::{collections::HashMap, str::FromStr};

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;

use super::{authorize, HttpState};
use crate::error::BackendError;
use ivynet_database::{client_log::ClientLog, log::LogLevel, Client, Machine};

/// Grab grab IDs for every machine under every client in the organization
#[utoipa::path(
    get,
    path = "/client",
    responses(
        (status = 200, body = [Info]),
        (status = 404)
    )
)]
pub async fn client(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<(Client, Vec<Machine>)>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let clients = account.clients_and_machines(&state.pool).await?;

    Ok(Json(clients))
}

/// Grab information on machines from a specific client id
#[utoipa::path(
    get,
    path = "/client/:id",
    responses(
        (status = 200, body = [Info]),
        (status = 404)
    )
)]
pub async fn client_machines(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Path(id): Path<String>,
) -> Result<Json<(Client, Vec<Machine>)>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let client = authorize::verify_client_ownership(&account, &state.pool, &id).await?;
    let machines = Machine::get_all_for_client_id(&state.pool, &client.client_id).await?;

    Ok(Json((client, machines)))
}

#[utoipa::path(
    get,
    path = "/client/:id/logs",
    responses(
        (status = 200, body = [ClientLog]),
        (status = 404)
    ),
    params(
        ("log_level" = Option<String>, Query, description = "Optional log level filter. Valid values: debug, info, warning, error"),
        ("from" = Option<i64>, Query, description = "Optional start timestamp"),
        ("to" = Option<i64>, Query, description = "Optional end timestamp")
    )
)]
pub async fn client_logs(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<ClientLog>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let client = authorize::verify_client_ownership(&account, &state.pool, &id).await?;

    let log_level = params
        .get("log_level")
        .map(|level| {
            LogLevel::from_str(level).map_err(|_| {
                BackendError::MalformedParameter("log_level".to_string(), level.clone())
            })
        })
        .transpose()?;

    let from = params.get("from").map(|s| s.parse::<i64>()).transpose().map_err(|_| {
        BackendError::MalformedParameter("from".to_string(), "Invalid timestamp".to_string())
    })?;
    let to = params.get("to").map(|s| s.parse::<i64>()).transpose().map_err(|_| {
        BackendError::MalformedParameter("to".to_string(), "Invalid timestamp".to_string())
    })?;

    if from.is_some() != to.is_some() {
        return Err(BackendError::MalformedParameter(
            "from/to".to_string(),
            "Both parameters must be present when querying by timestamp".to_string(),
        ));
    }

    let logs = ClientLog::get_all_for_client_with_filters(
        &state.pool,
        client.client_id,
        from,
        to,
        log_level,
    )
    .await?;

    Ok(Json(logs))
}
