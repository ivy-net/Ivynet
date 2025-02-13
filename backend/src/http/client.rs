use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;

use super::{authorize, HttpState};
use crate::error::BackendError;
use db::{Client, Machine};

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
