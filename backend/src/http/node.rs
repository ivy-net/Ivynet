use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use axum_extra::extract::CookieJar;
use ivynet_core::ethers::types::Address;

use crate::{
    data::node_data::{build_avs_info, AvsInfo, NodeStatusReport},
    db::{metric::Metric, Avs},
    error::BackendError,
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
        let info = build_avs_info(&state.pool, avs, metrics).await;
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
        let avs_info = build_avs_info(&state.pool, avs.clone(), node_metrics_map).await;
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

/// Delete all data for a specific AVS running on a node
#[utoipa::path(
    delete,
    path = "/avs/:id/:avs_name/:operator_id",
    responses(
        (status = 200),
        (status = 404)
    )
)]
pub async fn delete_avs_node_data(
    headers: HeaderMap,
    State(state): State<HttpState>,
    Path((id, avs_name, operator_id)): Path<(String, String, String)>,
    jar: CookieJar,
) -> Result<(), BackendError> {
    let account = authorize::verify(&state.pool, &headers, &state.cache, &jar).await?;
    let machine = authorize::verify_node_ownership(&account, State(state.clone()), id).await?;

    let op_id = operator_id.parse::<Address>().map_err(|_| BackendError::BadId)?;

    Avs::delete_avs_data(&state.pool, machine.machine_id, &op_id, &avs_name).await?;

    Ok(())
}
