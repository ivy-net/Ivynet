use crate::error::BackendError;
use axum::{extract::State, http::HeaderMap, Json};
use axum_extra::extract::CookieJar;
use ivynet_database::{
    data::node_data::{
        build_avs_info, get_active_set_information, ActiveSetInfo, AvsInfo, NodeStatusReport,
    },
    metric::Metric,
    operator_keys::OperatorKey,
};

use super::{authorize, HttpState};

/// Grab information for every node in the organization
#[utoipa::path(
    get,
    path = "/avs",
    responses(
        (status = 200, body = [Info]),
        (status = 404)
    )
)]
pub async fn all_avs_info(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<AvsInfo>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let avses = account.all_avses(&state.pool).await?;

    let mut info_reports: Vec<AvsInfo> = vec![];

    for avs in avses {
        let metrics =
            Metric::get_organized_for_avs(&state.pool, avs.machine_id, &avs.avs_name.to_string())
                .await?;
        let info = build_avs_info(&state.pool, avs, metrics).await?;
        info_reports.push(info);
    }

    Ok(Json(info_reports))
}

/// Get an overview of which nodes are healthy and unhealthy
#[utoipa::path(
    get,
    path = "/avs/status",
    responses(
        (status = 200, body = Status),
        (status = 404)
    )
)]
pub async fn avs_status(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<NodeStatusReport>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;

    let avses = account.all_avses(&state.pool).await?;

    let mut unhealthy_list: Vec<String> = vec![];
    let mut healthy_list: Vec<String> = vec![];

    for avs in &avses {
        let node_metrics_map =
            Metric::get_organized_for_avs(&state.pool, avs.machine_id, &avs.avs_name.to_string())
                .await?;
        let avs_info = build_avs_info(&state.pool, avs.clone(), node_metrics_map).await?;
        if !avs_info.errors.is_empty() {
            unhealthy_list.push(avs.avs_name.clone());
        } else {
            healthy_list.push(avs.avs_name.clone());
        }
    }

    Ok(Json(NodeStatusReport {
        total_nodes: avses.len(),
        healthy_nodes: healthy_list,
        unhealthy_nodes: unhealthy_list,
    }))
}

/// Get AVSes that you are not running a machine but have history in the active set
#[utoipa::path(
    get,
    path = "/avs/active_set",
    responses(
        (status = 200, body = [(OperatorKey, [ActiveSetInfo])]),
        (status = 404)
    )
)]
pub async fn avs_active_set(
    headers: HeaderMap,
    State(state): State<HttpState>,
    jar: CookieJar,
) -> Result<Json<Vec<(OperatorKey, Vec<ActiveSetInfo>)>>, BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let key_infos =
        OperatorKey::get_all_keys_for_organization(&state.pool, account.organization_id).await?;

    let active_set_info = get_active_set_information(&state.pool, key_infos).await?;

    Ok(Json(active_set_info))
}
