use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use ivynet_core::avs::names::AvsName;
use semver::Version;

use crate::{db::avs_data::DbAvsData, error::BackendError};

use super::{authorize, HttpState};

/// Get the latest version of an avs
#[utoipa::path(
    get,
    path = "/avs/:avs/version",
    responses(
        (status = 200, body = Metric),
        (status = 404)
    )
)]
pub async fn get_node_data_for_avs(
    headers: HeaderMap,
    State(state): State<HttpState>,
    Path(avs): Path<String>,
    jar: CookieJar,
) -> Result<Json<Version>, BackendError> {
    let _account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let avs_name = AvsName::from(&avs);

    // Get all data for the node
    let avs_data = DbAvsData::get_avs_data(&state.pool, &avs_name).await?;

    if let Some(data) = avs_data {
        Ok(Json(data.avs_version))
    } else {
        Err(BackendError::NoRunningAvsFound("AVS is not tracked yet".to_owned()))
    }
}
