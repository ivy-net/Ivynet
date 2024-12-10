use ivynet_core::{directory::avs_contract, node_type::NodeType};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use std::collections::HashMap;

use ivynet_core::ethers::types::Chain;

use semver::Version;

use crate::{
    db::{
        avs_version::{DbAvsVersionData, NodeTypeId, VersionData},
        metric::Metric,
        Avs, AvsActiveSet, AvsVersionHash,
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
    #[serde(flatten)]
    pub avs: Avs,
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
    mut avs: Avs,
    metrics: HashMap<String, Metric>,
) -> Result<AvsInfo, BackendError> {
    let running_metric = metrics.get(RUNNING_METRIC);

    let version_map = DbAvsVersionData::get_all_avs_version(pool).await;

    //Start of error building
    let mut errors = vec![];

    let active_set = if let (Some(address), Some(chain)) = (avs.operator_address, avs.chain) {
        if let Some(directory) = avs_contract(avs.avs_type, chain) {
            AvsActiveSet::get_active_set(pool, directory, address, chain).await.unwrap_or(false)
        } else {
            false
        }
    } else {
        false
    };

    avs.active_set = active_set;
    Avs::update_active_set(pool, avs.machine_id, &avs.avs_name, active_set).await?;

    if running_metric.is_none() {
        //Running metric missing should never really happen
        errors.push(NodeError::CrashedNode);

        //But if it does and you're in the active set, flag
        if active_set {
            errors.push(NodeError::ActiveSetNoDeployment);
        }
    }

    if let Some(run_met) = running_metric {
        //If running metric is not 1, the node has crashed
        if run_met.value == 1.0 {
            if !active_set {
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
            if active_set {
                errors.push(NodeError::ActiveSetNoDeployment);
            }
        }

        if run_met.value > 0.0 {}
    }

    let mut update_status = UpdateStatus::Unknown;
    if avs.chain.is_none() {
        errors.push(NodeError::NoChainInfo);
    } else if let Some(chain) = avs.chain {
        if let Ok(version_map) = version_map {
            update_status = get_update_status(
                version_map,
                avs.avs_version.clone(),
                Some(chain.to_string()),
                avs.avs_type,
            );
            if update_status == UpdateStatus::Outdated || update_status == UpdateStatus::Updateable
            {
                errors.push(NodeError::NeedsUpdate);
            }
        }
    }

    if avs.operator_address.is_none() {
        errors.push(NodeError::NoOperatorId);
    }

    Ok(AvsInfo {
        avs,
        uptime: metrics.get(UPTIME_METRIC).map_or(0.0, |m| m.value),
        performance_score: metrics.get(EIGEN_PERFORMANCE_METRIC).map_or(0.0, |m| m.value),
        update_status,
        errors,
    })
}

pub fn get_update_status(
    version_map: HashMap<NodeTypeId, VersionData>,
    avs_version: Version,
    chain: Option<String>,
    node_type: NodeType,
) -> UpdateStatus {
    // Early return if chain is missing
    let chain = match chain.and_then(|c| c.parse::<Chain>().ok()) {
        Some(c) => c,
        None => return UpdateStatus::Unknown,
    };

    // Get version data for this node type and chain
    let version_data = match version_map.get(&NodeTypeId { node_type, chain }) {
        Some(data) => data,
        None => return UpdateStatus::Unknown,
    };

    // Determine update status
    if version_data.breaking_change_version.as_ref().is_some_and(|breaking| avs_version < *breaking)
    {
        UpdateStatus::Outdated
    } else if version_data.latest_version > avs_version {
        UpdateStatus::Updateable
    } else {
        UpdateStatus::UpToDate
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

pub async fn update_avs_version(
    pool: &sqlx::PgPool,
    machine_id: Uuid,
    avs_name: &str,
    version_hash: &str,
) -> Result<(), BackendError> {
    let version = AvsVersionHash::get_version(pool, version_hash).await?;
    Avs::update_version(pool, machine_id, avs_name, &version).await?;
    Ok(())
}

#[cfg(test)]
mod node_data_tests {
    use super::*;

    fn create_test_version_map() -> HashMap<NodeTypeId, VersionData> {
        let mut map = HashMap::new();

        // Add EigenDA test data
        map.insert(
            NodeTypeId { node_type: NodeType::EigenDA, chain: Chain::Mainnet },
            VersionData {
                latest_version: Version::new(1, 2, 0),
                breaking_change_version: Some(Version::new(1, 0, 0)),
                breaking_change_datetime: None,
            },
        );

        map
    }

    #[test]
    fn test_update_status_up_to_date() {
        let version_map = create_test_version_map();
        let status = get_update_status(
            version_map,
            Version::new(1, 2, 0),
            Some("mainnet".to_string()),
            NodeType::EigenDA,
        );
        assert_eq!(status, UpdateStatus::UpToDate);
    }

    #[test]
    fn test_update_status_updateable() {
        let version_map = create_test_version_map();
        let status = get_update_status(
            version_map,
            Version::new(1, 1, 0),
            Some("mainnet".to_string()),
            NodeType::EigenDA,
        );
        assert_eq!(status, UpdateStatus::Updateable);
    }

    #[test]
    fn test_update_status_outdated() {
        let version_map = create_test_version_map();
        let status = get_update_status(
            version_map,
            Version::new(0, 9, 0),
            Some("mainnet".to_string()),
            NodeType::EigenDA,
        );
        assert_eq!(status, UpdateStatus::Outdated);
    }

    #[test]
    fn test_update_status_unknown_chain_or_type() {
        let version_map = create_test_version_map();

        // Test with invalid chain
        let status = get_update_status(
            version_map.clone(),
            Version::new(1, 2, 0),
            Some("invalid_chain".to_string()),
            NodeType::EigenDA,
        );
        assert_eq!(status, UpdateStatus::Unknown);

        // Test with unknown node type
        let status = get_update_status(
            version_map,
            Version::new(1, 2, 0),
            Some("mainnet".to_string()),
            NodeType::Unknown,
        );
        assert_eq!(status, UpdateStatus::Unknown);
    }

    #[test]
    fn test_update_status_missing_data() {
        let version_map = create_test_version_map();

        // Test with missing chain
        let status =
            get_update_status(version_map.clone(), Version::new(1, 2, 0), None, NodeType::EigenDA);
        assert_eq!(status, UpdateStatus::Unknown);

        // Test with empty version map
        let empty_map = HashMap::new();
        let status = get_update_status(
            empty_map,
            Version::new(1, 2, 0),
            Some("mainnet".to_string()),
            NodeType::EigenDA,
        );
        assert_eq!(status, UpdateStatus::Unknown);
    }
}
