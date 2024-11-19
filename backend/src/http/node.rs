use std::collections::{HashMap, HashSet};

use axum::{extract::State, http::HeaderMap, Json};
use axum_extra::extract::CookieJar;

use ivynet_core::{ethers::types::Chain, node_type::NodeType};
use semver::Version;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    data::{self},
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
    pub name: NodeType,
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
    pub chain: Option<String>,
    pub version: Option<String>,
    pub active_set: Option<String>,
    pub operator_id: Option<String>,
    pub uptime: f64,
    pub performance_score: f64,
    pub updateable: Option<bool>,
    pub outdated: Option<bool>,
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
    path = "/nodes/status",
    responses(
        (status = 200, body = Status),
        (status = 404)
    )
)]
pub async fn status(
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

    //TODO: Update these bits
    let (running_nodes, idle_nodes) = data::categorize_running_nodes(node_metrics_map.clone());
    let (healthy_nodes, low_perf_nodes) =
        data::categorize_node_health(running_nodes.clone(), node_metrics_map.clone());

    let avs_version_map = DbAvsVersionData::get_all_avs_version(&state.pool).await?;

    let (updateable_nodes, outdated_nodes) =
        data::categorize_updateable_nodes(running_nodes.clone(), node_metrics_map, avs_version_map);

    let mut unhealthy_list: HashSet<Uuid> = HashSet::new();
    unhealthy_list.extend(idle_nodes);
    unhealthy_list.extend(low_perf_nodes);
    unhealthy_list.extend(updateable_nodes);
    unhealthy_list.extend(outdated_nodes);

    Ok(Json(NodeStatusReport {
        total_nodes: avses.len(),
        healthy_nodes: healthy_nodes.iter().map(|node| format!("{node:?}")).collect(),
        unhealthy_nodes: unhealthy_list.into_iter().map(|node| node.to_string()).collect(),
    }))
}

//TODO: THIS WILL PROBABLY CHANGE ONCE CLIENT IMPL IS DONE
pub async fn build_avs_info(
    pool: &sqlx::PgPool,
    avs: Avs,
    metrics: HashMap<String, Metric>,
) -> AvsInfo {
    let running_metric = metrics.get(RUNNING_METRIC);
    let attrs = running_metric.and_then(|m| m.attributes.clone());
    let get_attr = |key| attrs.as_ref().and_then(|a| a.get(key).cloned());

    let name = get_attr("avs");
    let version = get_attr("version");
    let chain = get_attr("chain");

    //Like an onion
    let (updateable, outdated) = match (name.clone(), version.clone(), chain.clone()) {
        (Some(n), Some(v), Some(c)) => {
            let avs_name = Some(NodeType::from(n.as_str()));
            let avs_version = Version::parse(&v).ok();
            let avs_chain = c.parse::<Chain>().ok();

            match (avs_name, avs_version, avs_chain) {
                (Some(an), Some(current_version), Some(ac)) => {
                    if let Some(data) = DbAvsVersionData::get_avs_version_with_chain(pool, &an, &ac)
                        .await
                        .unwrap_or(None)
                    {
                        let outdated = data
                            .vd
                            .breaking_change_version
                            .map(|breaking| current_version < breaking)
                            .unwrap_or(false);

                        let updateable = data.vd.latest_version > current_version;

                        (Some(updateable), Some(outdated))
                    } else {
                        (None, None)
                    }
                }
                _ => (None, None),
            }
        }
        _ => (None, None),
    };

    AvsInfo {
        name,
        version,
        chain,
        active_set: get_attr("active_set"),
        operator_id: get_attr("operator_id"),
        uptime: metrics.get(UPTIME_METRIC).map_or(0.0, |m| m.value),
        performance_score: metrics.get(EIGEN_PERFORMANCE_METRIC).map_or(0.0, |m| m.value),
        updateable,
        machine_id: avs.machine_id.to_string(),
        outdated,
        errors: vec![], //FIXME: Add active set checking and operator key handling here
    }
}
