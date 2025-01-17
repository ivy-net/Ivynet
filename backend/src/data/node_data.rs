use ivynet_core::directory::avs_contract;
use ivynet_docker::{registry::ImageRegistry, RegistryType};
use ivynet_node_type::NodeType;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use std::collections::HashMap;

use ivynet_core::ethers::types::Chain;

use crate::{
    db::{
        avs_version::{DbAvsVersionData, NodeTypeId, VersionData},
        metric::Metric,
        Avs, AvsActiveSet, AvsVersionHash,
    },
    error::BackendError,
};

use super::avs_version::{extract_semver, VersionType};

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
    NoChainInfo,
    NoMetrics,
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
    avs: Avs,
    metrics: HashMap<String, Metric>,
) -> Result<AvsInfo, BackendError> {
    let mut avs = avs;
    let metrics_alive = avs.metrics_alive;

    let version_map = DbAvsVersionData::get_all_avs_version(pool).await;

    //Start of error building
    let mut errors = vec![];

    if !avs.active_set {
        errors.push(NodeError::UnregisteredFromActiveSet);
    }

    if let Some(datetime) = avs.updated_at {
        let now = chrono::Utc::now().naive_utc();
        if now.signed_duration_since(datetime).num_minutes() > IDLE_MINUTES_THRESHOLD {
            errors.push(NodeError::CrashedNode);

            if avs.active_set {
                errors.push(NodeError::ActiveSetNoDeployment);
            }
        }
    }

    if !metrics_alive {
        errors.push(NodeError::NoMetrics);
    }

    let mut update_status = UpdateStatus::Unknown;
    if avs.chain.is_none() {
        errors.push(NodeError::NoChainInfo);
    } else if let Some(chain) = avs.chain {
        if let Ok(version_map) = version_map {
            update_status = get_update_status(
                version_map,
                &avs.avs_version,
                &avs.version_hash,
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

    if avs.avs_type.registry() == Ok(RegistryType::Othentic) {
        avs.avs_version = "Othentic".to_string();
    } else if avs.avs_type.registry() == Ok(RegistryType::Local) {
        avs.avs_version = "Local".to_string();
    }

    Ok(AvsInfo {
        avs,
        uptime: metrics.get(UPTIME_METRIC).map_or(0.0, |m| m.value),
        performance_score: metrics.get(EIGEN_PERFORMANCE_METRIC).map_or(0.0, |m| m.value),
        update_status,
        errors,
    })
}

/// node_version_tag: corresponds to the docker image tag for the node.
/// node_image_digest: corresponds to the docker image digest for the node.
pub fn get_update_status(
    version_map: HashMap<NodeTypeId, VersionData>,
    node_version_tag: &str,
    node_image_digest: &str,
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

    match VersionType::from(&node_type) {
        VersionType::SemVer => {
            let latest_semver = match extract_semver(&version_data.latest_version) {
                Some(semver) => semver,
                None => return UpdateStatus::Unknown,
            };

            let query_semver = match extract_semver(node_version_tag) {
                Some(semver) => semver,
                None => return UpdateStatus::Unknown,
            };

            let breaking_change_semver = match version_data.breaking_change_version.as_ref() {
                Some(breaking_change) => extract_semver(&breaking_change.to_string()),
                None => None,
            };

            if let Some(breaking_change_semver) = breaking_change_semver {
                if query_semver < breaking_change_semver {
                    return UpdateStatus::Outdated;
                }
            }

            if query_semver >= latest_semver {
                return UpdateStatus::UpToDate;
            }

            UpdateStatus::Updateable
        }
        // TODO: This is pretty dumb at the moment, no real way to check for breaking change
        // versions for fixed versions
        VersionType::FixedVer | VersionType::HybridVer => {
            if node_image_digest == version_data.latest_version_digest {
                return UpdateStatus::UpToDate;
            }
            UpdateStatus::Updateable
        }
        VersionType::LocalOnly => UpdateStatus::Unknown,
    }
}

/// Condense list of metrics into a smaller list of metrics for the frontend
pub fn condense_metrics(node_type: NodeType, metrics: &[Metric]) -> Vec<Metric> {
    match node_type {
        NodeType::EigenDA => filter_metrics_by_names(metrics, &CONDENSED_EIGENDA_METRICS_NAMES),
        _ => metrics.to_vec(), /* If we haven't implemented a condensed metrics list for this
                                * node type, just return all metrics */
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
    digest: &str,
) -> Result<(), BackendError> {
    let version = AvsVersionHash::get_version(pool, digest).await?;
    Avs::update_version(pool, machine_id, avs_name, &version, digest).await?;

    Ok(())
}

pub async fn update_avs_active_set(
    pool: &sqlx::PgPool,
    machine_id: Uuid,
    avs_name: &str,
) -> Result<(), BackendError> {
    let avs = Avs::get_machines_avs(pool, machine_id, avs_name).await?;

    let active_set = if let Some(avs) = avs {
        if let (Some(address), Some(chain)) = (avs.operator_address, avs.chain) {
            if let Some(directory) = avs_contract(avs.avs_type, chain) {
                AvsActiveSet::get_active_set(pool, directory, address, chain).await.unwrap_or(false)
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    Avs::update_active_set(pool, machine_id, avs_name, active_set).await?;
    Ok(())
}

// TODO: These need to also text fixed versions
#[cfg(test)]
mod node_data_tests {
    use semver::Version;

    use super::*;

    fn create_test_version_map() -> HashMap<NodeTypeId, VersionData> {
        let mut map = HashMap::new();

        // Add EigenDA test data
        map.insert(
            NodeTypeId { node_type: NodeType::EigenDA, chain: Chain::Mainnet },
            VersionData {
                latest_version: Version::new(1, 2, 0).to_string(),
                latest_version_digest: "digest".to_string(),
                breaking_change_version: Some(Version::new(1, 0, 0).to_string()),
                breaking_change_datetime: None,
            },
        );

        map
    }

    #[test]
    fn test_update_status_up_to_date() {
        let version_map = create_test_version_map();
        let test_digest = "digest";
        let status = get_update_status(
            version_map,
            Version::new(1, 2, 0).to_string().as_ref(),
            test_digest,
            Some("mainnet".to_string()),
            NodeType::EigenDA,
        );
        assert_eq!(status, UpdateStatus::UpToDate);
    }

    #[test]
    fn test_update_status_updateable() {
        let version_map = create_test_version_map();
        let test_digest = "different_digest";
        let status = get_update_status(
            version_map,
            Version::new(1, 1, 0).to_string().as_ref(),
            test_digest,
            Some("mainnet".to_string()),
            NodeType::EigenDA,
        );
        assert_eq!(status, UpdateStatus::Updateable);
    }

    #[test]
    fn test_update_status_outdated() {
        let version_map = create_test_version_map();
        let test_digest = "different_digest";
        let status = get_update_status(
            version_map,
            Version::new(0, 9, 0).to_string().as_ref(),
            test_digest,
            Some("mainnet".to_string()),
            NodeType::EigenDA,
        );
        assert_eq!(status, UpdateStatus::Outdated);
    }

    #[test]
    fn test_update_status_unknown_chain_or_type() {
        let version_map = create_test_version_map();
        let test_digest = "digest";
        // Test with invalid chain
        let status = get_update_status(
            version_map.clone(),
            Version::new(1, 2, 0).to_string().as_ref(),
            test_digest,
            Some("invalid_chain".to_string()),
            NodeType::EigenDA,
        );
        assert_eq!(status, UpdateStatus::Unknown);

        // Test with unknown node type
        let status = get_update_status(
            version_map,
            Version::new(1, 2, 0).to_string().as_ref(),
            test_digest,
            Some("mainnet".to_string()),
            NodeType::Unknown,
        );
        assert_eq!(status, UpdateStatus::Unknown);
    }

    #[test]
    fn test_update_status_missing_data() {
        let version_map = create_test_version_map();
        let test_digest = "digest";

        // Test with missing chain
        let status = get_update_status(
            version_map.clone(),
            Version::new(1, 2, 0).to_string().as_ref(),
            test_digest,
            None,
            NodeType::EigenDA,
        );
        assert_eq!(status, UpdateStatus::Unknown);

        // Test with empty version map
        let empty_map = HashMap::new();
        let status = get_update_status(
            empty_map,
            Version::new(1, 2, 0).to_string().as_ref(),
            test_digest,
            Some("mainnet".to_string()),
            NodeType::EigenDA,
        );
        assert_eq!(status, UpdateStatus::Unknown);
    }
}
