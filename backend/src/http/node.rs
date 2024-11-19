use std::collections::{HashMap, HashSet};

use axum::{extract::State, http::HeaderMap, Json};
use axum_extra::extract::CookieJar;

use ivynet_core::node_type::NodeType;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    data::{self, get_update_status, UpdateStatus},
    db::{avs::Avs, avs_version::DbAvsVersionData, metric::Metric},
    error::BackendError,
};

use super::{authorize, HttpState};

#[derive(Serialize, Debug, Clone)]
#[allow(dead_code)]
pub enum NodeError {
    NoOperatorId,
    ActiveSetNoDeployment,
    UnregisteredFromActiveSet,
}

#[derive(Serialize, Debug, Clone)]
pub struct NodeErrorInfo {
    pub name: String,
    pub node_type: NodeType,
    pub errors: Vec<NodeError>,
}

#[derive(Serialize, ToSchema, Clone, Debug, Default)]
pub struct NodeStatusReport {
    pub total_nodes: usize,
    pub healthy_nodes: Vec<String>,
    pub unhealthy_nodes: Vec<String>,
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct AvsInfo {
    pub machine_id: String,
    pub name: Option<String>,
    pub node_type: Option<String>,
    pub chain: Option<String>,
    pub version: Option<String>,
    pub active_set: Option<String>,
    pub operator_id: Option<String>,
    pub uptime: f64,
    pub performance_score: f64,
    pub update_status: UpdateStatus,
    pub errors: Vec<NodeError>,
}

const UPTIME_METRIC: &str = "uptime";
const RUNNING_METRIC: &str = "running";
const EIGEN_PERFORMANCE_METRIC: &str = "eigen_performance_score";

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

    //Hashmap of node_id to metrics
    let mut node_metrics_map: HashMap<Uuid, HashMap<String, Metric>> = HashMap::new();

    for avs in &avses {
        node_metrics_map.insert(
            avs.machine_id,
            Metric::get_organized_for_avs(&state.pool, avs.machine_id, &avs.avs_name.to_string())
                .await?,
        );
    }

    let (running_nodes, idle_nodes) = data::categorize_running_nodes(node_metrics_map.clone());
    let (healthy_nodes, low_perf_nodes) =
        data::categorize_node_health(running_nodes.clone(), node_metrics_map.clone());

    let updateable_nodes = data::categorize_updateable_nodes(
        DbAvsVersionData::get_all_avs_version(&state.pool).await?,
        running_nodes.clone(),
        node_metrics_map,
    );

    let mut unhealthy_list: HashSet<Uuid> = HashSet::new();
    unhealthy_list.extend(idle_nodes);
    unhealthy_list.extend(low_perf_nodes);
    unhealthy_list.extend(updateable_nodes.iter().map(|node| node.0));

    Ok(Json(NodeStatusReport {
        total_nodes: avses.len(),
        healthy_nodes: healthy_nodes.iter().map(|node| format!("{node:?}")).collect(),
        unhealthy_nodes: unhealthy_list.into_iter().map(|node| node.to_string()).collect(),
    }))
}

pub async fn build_avs_info(
    pool: &sqlx::PgPool,
    avs: Avs,
    metrics: HashMap<String, Metric>,
) -> AvsInfo {
    let running_metric = metrics.get(RUNNING_METRIC);
    let attrs = running_metric.and_then(|m| m.attributes.clone());
    let get_attr = |key| attrs.as_ref().and_then(|a| a.get(key).cloned());

    let name = get_attr("avs_name");
    let node_type = get_attr("avs_type");
    let version = get_attr("version");
    let chain = get_attr("chain");

    let version_map = DbAvsVersionData::get_all_avs_version(pool).await;

    let mut update_status = UpdateStatus::Unknown;
    if let Ok(version_map) = version_map {
        update_status =
            get_update_status(version_map, version.clone(), chain.clone(), node_type.clone());
    }

    AvsInfo {
        name,
        node_type,
        version,
        chain,
        active_set: None, //FIXME: Add active set checking and operator key handling here
        operator_id: None, //FIXME: Add active set checking and operator key handling here
        uptime: metrics.get(UPTIME_METRIC).map_or(0.0, |m| m.value),
        performance_score: metrics.get(EIGEN_PERFORMANCE_METRIC).map_or(0.0, |m| m.value),
        update_status,
        machine_id: avs.machine_id.to_string(),
        errors: vec![], //FIXME: Add active set checking and operator key handling here
    }
}
