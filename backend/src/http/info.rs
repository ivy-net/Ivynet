use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use ivynet_core::node_type::NodeType;
use tracing::debug;

use crate::{
    db::{avs_version::DbAvsVersionData, AvsVersionData},
    error::BackendError,
};

use super::{authorize, HttpState};

/// Get the latest version of an avs
#[utoipa::path(
    get,
    path = "/info/avs/version/:avs",
    responses(
        (status = 200, body = Metric),
        (status = 404)
    )
)]
pub async fn get_version_info(
    headers: HeaderMap,
    State(state): State<HttpState>,
    Path(avs): Path<String>,
    jar: CookieJar,
) -> Result<Json<Vec<AvsVersionData>>, BackendError> {
    let _account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let avs_name = NodeType::from(avs.as_str());

    if let NodeType::Unknown(_) = avs_name {
        return Err(BackendError::InvalidAvs)
    }

    let avs_data = DbAvsVersionData::get_avs_version(&state.pool, &avs_name).await?;

    Ok(Json(avs_data))
}

/// Get the latest version for every AVS we support
#[utoipa::path(
    get,
    path = "/info/avs/version",
    responses(
        (status = 200, body = Metric),
        (status = 404)
    )
)]
pub async fn get_all_version_info(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<AvsVersionData>>, BackendError> {
    let _account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    // Get all data for the node
    let avs_data = DbAvsVersionData::get_all_avs_version(&state.pool).await?;

    let vec_data: Vec<AvsVersionData> =
        avs_data.into_iter().map(|(id, vd)| AvsVersionData { id, vd }).collect();

    debug!("/info/avs/version result: {:#?}", vec_data);

    Ok(Json(vec_data))
}
