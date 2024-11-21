use std::collections::{HashMap, HashSet};

use axum::{extract::State, http::HeaderMap, Json};
use axum_extra::extract::CookieJar;

use ivynet_core::node_type::NodeType;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    data::{
        self, get_update_status, UpdateStatus, EIGEN_PERFORMANCE_HEALTHY_THRESHOLD,
        IDLE_MINUTES_THRESHOLD,
    },
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
    LowPerformanceScore,
    HardwareResourceUsage,
    NeedsUpdate,
    CrashedNode,
    IdleNodeNoCommunication,
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
    pub active_set: Option<bool>,
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

    let mut unhealthy_list: Vec<Uuid> = vec![];
    let mut healthy_list: Vec<Uuid> = vec![];

    for avs in &avses {
        let node_metrics_map =
            Metric::get_organized_for_avs(&state.pool, avs.machine_id, &avs.avs_name.to_string())
                .await?;
        let avs_info = build_avs_info(&state.pool, avs.clone(), node_metrics_map).await;
        if avs_info.errors.len() > 0 {
            unhealthy_list.push(avs.machine_id);
        } else {
            healthy_list.push(avs.machine_id);
        }
    }

    Ok(Json(NodeStatusReport {
        total_nodes: avses.len(),
        healthy_nodes: healthy_list.iter().map(|node| format!("{node:?}")).collect(),
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

    //Start of error building
    let mut errors = vec![];

    if running_metric.is_none() {
        //Running metric missing should never really happen
        errors.push(NodeError::CrashedNode);

        //But if it does and you're in the active set, flag
        if avs.active_set {
            errors.push(NodeError::ActiveSetNoDeployment);
        }
    }

    if let Some(run_met) = running_metric {
        //If running metric is not 1, the node has crashed
        if run_met.value == 1.0 {
            if !avs.active_set {
                errors.push(NodeError::UnregisteredFromActiveSet);
            }

            if let Some(perf) = metrics.get(EIGEN_PERFORMANCE_METRIC) {
                if perf.value < EIGEN_PERFORMANCE_HEALTHY_THRESHOLD {
                    errors.push(NodeError::LowPerformanceScore);
                }
            }

            if let Some(datetime) = run_met.created_at {
                let now = chrono::Utc::now().naive_utc();
                if now.signed_duration_since(datetime).num_minutes() < IDLE_MINUTES_THRESHOLD {
                    errors.push(NodeError::IdleNodeNoCommunication);
                }
            }
        } else {
            errors.push(NodeError::CrashedNode);

            //In active set but not running a node could be inactivity slashable
            if avs.active_set {
                errors.push(NodeError::ActiveSetNoDeployment);
            }
        }

        if run_met.value > 0.0 {}
    }

    let mut update_status = UpdateStatus::Unknown;
    if let Ok(version_map) = version_map {
        update_status =
            get_update_status(version_map, version.clone(), chain.clone(), node_type.clone());
        if update_status != UpdateStatus::UpToDate {
            errors.push(NodeError::NeedsUpdate);
        }
    }

    if avs.operator_address.is_none() {
        errors.push(NodeError::NoOperatorId);
    }

    AvsInfo {
        name,
        node_type,
        version,
        chain,
        active_set: Some(avs.active_set), /* FIXME: Add active set checking and operator key
                                           * handling here */
        operator_id: None, //FIXME: Add active set checking and operator key handling here
        uptime: metrics.get(UPTIME_METRIC).map_or(0.0, |m| m.value),
        performance_score: metrics.get(EIGEN_PERFORMANCE_METRIC).map_or(0.0, |m| m.value),
        update_status,
        machine_id: avs.machine_id.to_string(),
        errors,
    }
}
