use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;

use super::{authorize, HttpState};
use crate::error::BackendError;

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
    let account = authorize::verify(&state.database, &headers, &state.cache, &jar).await?;
    let clients = state
        .database
        .get_clients_for_account(Request::new(Id { id: account.id }))
        .await?
        .into_inner();

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
    let address = id.parse::<Address>().map_err(|_| BackendError::BadId)?;
    let client = state
        .database
        .get_client(Request::new(ClientAndOwner {
            owner_id: account.id,
            client: address.as_bytes().to_bytes(),
        }))
        .await?
        .into_inner();
    let machines = state
        .database
        .get_machines_for_client(Request::new(Address { id: address.as_bytes().to_vec() }))
        .await?
        .into_inner();
    Ok(Json((client, machines)))
}
