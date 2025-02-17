use std::{collections::HashMap, sync::Arc};

use ivynet_core::ethers::types::Chain;
use ivynet_grpc::messages::NodeDataV2;
use ivynet_node_type::NodeType;
use ivynet_notifications::NotificationType;
use serde::{Deserialize, Serialize};
use sqlx::{types::Uuid, PgPool};

use crate::{
    avs_version::{NodeTypeId, VersionData},
    data::{
        avs_version::{extract_semver, VersionType},
        node_data::UpdateStatus,
    },
    error::DatabaseError,
    Avs, DbAvsVersionData,
};

use super::alerts_active::{ActiveAlert, NewAlert};

pub const RUNNING_METRIC: &str = "running";
pub const EIGEN_PERFORMANCE_METRIC: &str = "eigen_performance_score";

pub const IDLE_MINUTES_THRESHOLD: i64 = 15;
pub const EIGEN_PERFORMANCE_HEALTHY_THRESHOLD: f64 = 80.0;

pub enum UuidAlertType {
    NoMetrics(),
}

pub struct NoMetricsAlert {
    pub machine_id: Uuid,
    pub node_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AlertError {
    #[error(transparent)]
    DbError(#[from] DatabaseError),
}

#[derive(Debug, Clone)]
pub struct AlertHandler {
    db_executor: Arc<PgPool>,
}

impl AlertHandler {
    pub fn new(db_executor: Arc<PgPool>) -> Self {
        Self { db_executor }
    }

    pub async fn handle_node_data_alerts(
        &self,
        node_data: NodeDataV2,
        machine_id: Uuid,
    ) -> Result<(), AlertError> {
        let raw_alerts = extract_node_data_alerts(&self.db_executor, machine_id, &node_data).await;
        let alerts = raw_alerts
            .into_iter()
            .map(|alert| NewAlert::new(machine_id, alert, node_data.name.clone()))
            .collect::<Vec<_>>();
        ActiveAlert::insert_many(&self.db_executor, &alerts).await?;
        Ok(())
    }
}

async fn extract_node_data_alerts(
    pool: &PgPool,
    machine_id: Uuid,
    node_data: &NodeDataV2,
) -> Vec<NotificationType> {
    let mut alerts = vec![];

    // Necessary db calls to compare state

    let avs = if let Ok(Some(avs)) = Avs::get_machines_avs(pool, machine_id, &node_data.name).await
    {
        avs
    } else {
        return vec![];
    };

    let version_map = DbAvsVersionData::get_all_avs_version(pool).await;

    // extraction logic

    if let Some(datetime) = avs.updated_at {
        let now = chrono::Utc::now().naive_utc();
        if now.signed_duration_since(datetime).num_minutes() > IDLE_MINUTES_THRESHOLD {
            // alerts.push(NotificationType::CrashedNode);

            if avs.active_set {
                // alerts.push(NotificationType::ActiveSetNoDeployment);
            }
        }
    }

    if !node_data.metrics_alive() {
        alerts.push(NotificationType::NoMetrics(node_data.name.clone()));
    }

    if !node_data.node_running() {
        alerts.push(NotificationType::NodeNotRunning(node_data.name.clone()));
    }

    if avs.chain.is_none() {
        alerts.push(NotificationType::NoChainInfo(node_data.name.clone()));
    } else if let Some(chain) = avs.chain {
        if let Ok(version_map) = version_map {
            let update_status = get_update_status(
                version_map,
                &avs.avs_version,
                &avs.version_hash,
                Some(chain.to_string()),
                avs.avs_type,
            );
            if update_status == UpdateStatus::Outdated || update_status == UpdateStatus::Updateable
            {
                alerts.push(NotificationType::NeedsUpdate {
                    avs: node_data.name.clone(),
                    current_version: avs.avs_version.clone(),
                    recommended_version: todo!("Return recommended version from db"),
                });
            }
        }
    }

    if avs.operator_address.is_none() {
        alerts.push(NotificationType::NoOperatorId(node_data.name.clone()));
    }

    alerts
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
        VersionType::OptInOnly => UpdateStatus::Unknown,
    }
}
