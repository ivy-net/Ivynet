use ivynet_core::node_type::NodeType;
use serde::Serialize;
use utoipa::ToSchema;

use std::collections::HashMap;

use ivynet_core::ethers::types::Chain;

use semver::Version;

use crate::{
    db::{
        avs_version::{DbAvsVersionData, NodeTypeId, VersionData},
        metric::Metric,
        Avs,
    },
    error::BackendError,
};

const UPTIME_METRIC: &str = "uptime";
pub const RUNNING_METRIC: &str = "running";
pub const EIGEN_PERFORMANCE_METRIC: &str = "eigen_performance_score";

pub const IDLE_MINUTES_THRESHOLD: i64 = 15;
pub const EIGEN_PERFORMANCE_HEALTHY_THRESHOLD: f64 = 80.0;

const CONDENSED_EIGENDA_METRICS_NAMES: [&str; 2] =
    ["eigen_performance_score", "node_reachability_status"];

#[derive(Serialize, Debug, Clone)]
pub enum NodeError {
    NoOperatorId,
    ActiveSetNoDeployment,
    UnregisteredFromActiveSet,
    LowPerformanceScore,
    HardwareResourceUsage,
    NeedsUpdate,
    CrashedNode,
    IdleNodeNoCommunication,
    NoChainInfo,
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
    pub operator_id: Option<String>,
    pub active_set: bool,
    pub uptime: f64,
    pub performance_score: f64,
    pub update_status: UpdateStatus,
    pub errors: Vec<NodeError>,
}

#[derive(Serialize, ToSchema, Clone, Debug, PartialEq)]
pub enum UpdateStatus {
    Outdated,
    Updateable,
    UpToDate,
    Unknown,
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
                if now.signed_duration_since(datetime).num_minutes() > IDLE_MINUTES_THRESHOLD {
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
        if update_status != UpdateStatus::UpToDate || update_status != UpdateStatus::Unknown {
            errors.push(NodeError::NeedsUpdate);
        }
    }

    if avs.operator_address.is_none() {
        errors.push(NodeError::NoOperatorId);
    }

    if chain.is_none() {
        errors.push(NodeError::NoChainInfo);
    }

    AvsInfo {
        name,
        node_type,
        version,
        chain,
        active_set: avs.active_set, //Microservice should handle this
        operator_id: avs.operator_address.map(|addr| addr.to_string()),
        uptime: metrics.get(UPTIME_METRIC).map_or(0.0, |m| m.value),
        performance_score: metrics.get(EIGEN_PERFORMANCE_METRIC).map_or(0.0, |m| m.value),
        update_status,
        machine_id: avs.machine_id.to_string(),
        errors,
    }
}

pub fn get_update_status(
    version_map: HashMap<NodeTypeId, VersionData>,
    avs_version: Option<String>,
    chain: Option<String>,
    node_type: Option<String>,
) -> UpdateStatus {
    match (avs_version, chain, node_type) {
        (Some(v), Some(c), Some(nt)) => {
            let node_type = NodeType::from(nt.as_str());
            let avs_version = Version::parse(&v).ok();
            let avs_chain = c.parse::<Chain>().ok();

            match (avs_version, avs_chain) {
                (Some(current_version), Some(ac)) if node_type != NodeType::Unknown => {
                    if let Some(data) = version_map.get(&NodeTypeId { node_type, chain: ac }) {
                        if data
                            .breaking_change_version
                            .clone()
                            .map(|breaking| current_version < breaking)
                            .unwrap_or(false)
                        {
                            UpdateStatus::Outdated
                        } else if data.latest_version > current_version {
                            UpdateStatus::Updateable
                        } else {
                            UpdateStatus::UpToDate
                        }
                    } else {
                        UpdateStatus::Unknown
                    }
                }
                _ => UpdateStatus::Unknown,
            }
        }
        _ => UpdateStatus::Unknown,
    }
}

/// Condense list of metrics into a smaller list of metrics for the frontend
pub fn condense_metrics(
    node_type: NodeType,
    metrics: &[Metric],
) -> Result<Vec<Metric>, BackendError> {
    match node_type {
        NodeType::EigenDA => Ok(filter_metrics_by_names(metrics, &CONDENSED_EIGENDA_METRICS_NAMES)),
        _ => Err(BackendError::CondensedMetricsNotFound(format!(
            "No condensed metrics found for AVS: {}, use the /metrics/all endpoint instead",
            node_type
        ))),
    }
}

/// Filter the metrics by the given names.
fn filter_metrics_by_names(metrics: &[Metric], allowed_names: &[&str]) -> Vec<Metric> {
    metrics.iter().filter(|metric| allowed_names.contains(&metric.name.as_str())).cloned().collect()
}
