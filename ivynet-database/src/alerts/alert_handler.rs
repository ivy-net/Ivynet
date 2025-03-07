use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use ethers::types::Address;
use ivynet_alerts::Alert;
use ivynet_error::ethers::types::Chain;
use ivynet_grpc::messages::NodeDataV2;
use ivynet_node_type::NodeType;
use ivynet_notifications::{
    Channel, Notification, NotificationDispatcher, NotificationDispatcherError,
};

use sqlx::{types::Uuid, PgPool};

use crate::{
    avs_version::{NodeTypeId, VersionData},
    data::{
        avs_version::{extract_semver, VersionType},
        node_data::UpdateStatus,
    },
    eigen_avs_metadata::{EigenAvsMetadata, MetadataContent},
    error::DatabaseError,
    Avs, DbAvsVersionData, Machine, NotificationSettings,
};

use super::{
    alert_db::AlertDb,
    node_alerts_active::{NewAlert, NodeActiveAlert},
};

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
    #[error(transparent)]
    NotificationError(#[from] NotificationDispatcherError),
    #[error(transparent)]
    SqxlError(#[from] sqlx::Error),
}

#[derive(Clone)]
pub struct AlertHandler {
    pub dispatcher: Arc<NotificationDispatcher<AlertDb>>,
    db_executor: PgPool,
}

impl AlertHandler {
    pub fn new(dispatcher: Arc<NotificationDispatcher<AlertDb>>, db_executor: PgPool) -> Self {
        Self { dispatcher, db_executor }
    }

    pub async fn handle_node_data_alerts(
        &self,
        node_data: NodeDataV2,
        machine_id: Uuid,
    ) -> Result<(), AlertError> {
        // alert extraction and insertion
        let raw_alerts = extract_node_data_alerts(&self.db_executor, machine_id, &node_data).await;

        let new_alerts = raw_alerts
            .into_iter()
            .map(|alert| NewAlert::new(machine_id, alert, node_data.name.clone()))
            .collect::<Vec<_>>();

        let filtered_new_alerts = self.filter_duplicate_alerts(new_alerts).await?;

        NodeActiveAlert::insert_many(&self.db_executor, &filtered_new_alerts).await?;

        // Handle notification dispatch. Notification filtering happens post-insertion, so alerts
        // are still visible in the database.
        let organization_id =
            Machine::get_organization_id(&self.db_executor, machine_id).await? as u64;

        let (channels, alert_ids) = self.organization_channel_alerts(organization_id).await;

        let flag_filtered_alerts = filtered_new_alerts
            .into_iter()
            .filter(|alert| alert_ids.contains(&alert.alert_type.id()))
            .collect::<Vec<_>>();

        let notifications: Vec<Notification> = flag_filtered_alerts
            .into_iter()
            .map(|alert| Notification {
                id: alert.id,
                organization: organization_id,
                machine_id,
                alert: alert.alert_type,
                resolved: false,
            })
            .collect();

        for notification in notifications {
            self.dispatcher.notify(notification, channels.clone()).await?;
        }

        Ok(())
    }

    pub async fn handle_new_eigen_avs_alerts(
        &self,
        pool: &PgPool,
        avs_address: &Address,
        metadata_uri: &str,
        metadata_content: &MetadataContent,
    ) -> Result<(), AlertError> {
        let count = EigenAvsMetadata::search_for_avs(
            pool,
            *avs_address,
            metadata_uri.to_owned(),
            metadata_content.name.clone().unwrap_or_default(),
            metadata_content.website.clone().unwrap_or_default(),
            metadata_content.twitter.clone().unwrap_or_default(),
        )
        .await
        .map_err(|e| {
            AlertError::DbError(DatabaseError::FailedMetadata(format!(
                "Failed to get count of metadata: {}",
                e
            )))
        })?;

        println!("count: {}", count);
        println!("name: {}", metadata_content.name.clone().unwrap_or_default());
        println!("metadata_uri: {}", metadata_uri);

        if count > 0 {
            tracing::debug!("AVS already registered - sending update avs alert");
        } else {
            tracing::info!("AVS not registered - sending new avs alert");
        }
        println!("--------------------------------");

        Ok(())
    }

    // Filters duplicate incoming alerts by checking computed UUID against existing alerts in the
    // database. If the alert is already present, it is not included in the returned list.
    pub async fn filter_duplicate_alerts(
        &self,
        alerts: Vec<NewAlert>,
    ) -> Result<Vec<NewAlert>, AlertError> {
        let ids = alerts.iter().map(|alert| alert.id).collect::<Vec<_>>();

        let existing_ids: Vec<Uuid> = NodeActiveAlert::get_many(&self.db_executor, &ids)
            .await?
            .iter()
            .map(|alert| alert.alert_id)
            .collect();

        let filtered = alerts
            .into_iter()
            .filter(|alert| !existing_ids.contains(&alert.id))
            .collect::<Vec<_>>();

        Ok(filtered)
    }

    /// Returns the channels that are enabled for the organization, as well as flags for
    /// enabled/disabled alerts in the form of a vec.
    pub async fn organization_channel_alerts(
        &self,
        organization_id: u64,
    ) -> (HashSet<Channel>, Vec<usize>) {
        let mut channels = HashSet::new();
        let org_notifications = NotificationSettings::get(&self.db_executor, organization_id)
            .await
            .expect("Organization notifications not found");

        if org_notifications.telegram {
            channels.insert(Channel::Telegram);
        }
        if org_notifications.email {
            channels.insert(Channel::Email);
        }
        if org_notifications.pagerduty {
            channels.insert(Channel::PagerDuty);
        }

        (channels, org_notifications.alert_flags.to_alert_ids())
    }
}

async fn extract_node_data_alerts(
    pool: &PgPool,
    machine_id: Uuid,
    node_data: &NodeDataV2,
) -> Vec<Alert> {
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
            alerts.push(Alert::NodeNotResponding {
                node_name: node_data.name.clone(),
                node_type: avs.avs_type.to_string(),
            });
            if avs.active_set && avs.operator_address.is_some() {
                alerts.push(Alert::ActiveSetNoDeployment {
                    node_name: node_data.name.clone(),
                    node_type: avs.avs_type.to_string(),
                    operator: avs.operator_address.expect("UNENTERABLE"),
                });
            }
        }
    }

    if !node_data.metrics_alive() {
        alerts.push(Alert::NoMetrics {
            node_name: node_data.name.clone(),
            node_type: avs.avs_type.to_string(),
        });
    }

    if !node_data.node_running() {
        alerts.push(Alert::NodeNotRunning {
            node_name: node_data.name.clone(),
            node_type: avs.avs_type.to_string(),
        });
    }

    if avs.chain.is_none() {
        alerts.push(Alert::NoChainInfo {
            node_name: node_data.name.clone(),
            node_type: avs.avs_type.to_string(),
        });
    } else if let Some(chain) = avs.chain {
        if let Ok(version_map) = version_map {
            let update_status = get_update_status(
                version_map.clone(),
                &avs.avs_version,
                &avs.version_hash,
                Some(chain.to_string()),
                avs.avs_type,
            );

            let node_type_id = NodeTypeId { node_type: avs.avs_type, chain };

            if let Some(version_data) = version_map.get(&node_type_id) {
                let recommended_version = version_data.latest_version.clone();
                if update_status == UpdateStatus::Outdated ||
                    update_status == UpdateStatus::Updateable
                {
                    alerts.push(Alert::NeedsUpdate {
                        node_name: node_data.name.clone(),
                        node_type: avs.avs_type.to_string(),
                        current_version: avs.avs_version.clone(),
                        recommended_version,
                    });
                }
            }
        }
    }

    if avs.operator_address.is_none() {
        alerts.push(Alert::NoOperatorId {
            node_name: node_data.name.clone(),
            node_type: avs.avs_type.to_string(),
        });
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

#[cfg(test)]
mod tests {
    use ivynet_notifications::{NotificationConfig, SendgridSpecificTemplates, SendgridTemplates};

    use super::*;

    fn dummy_config_fixture() -> NotificationConfig {
        let specific_templates = SendgridSpecificTemplates {
            custom: "test".to_string(),
            unreg_active_set: "test".to_string(),
            machine_not_responding: "test".to_string(),
            node_not_running: "test".to_string(),
            no_chain_info: "test".to_string(),
            no_metrics: "test".to_string(),
            no_operator: "test".to_string(),
            hw_res_usage: "test".to_string(),
            low_perf: "test".to_string(),
            needs_update: "test".to_string(),
            new_eigen_avs: "test".to_string(),
            updated_eigen_avs: "test".to_string(),
        };

        NotificationConfig {
            telegram_token: "test".to_string(),
            sendgrid_key: "test".to_string(),
            sendgrid_from: "test".to_string(),
            sendgrid_templates: SendgridTemplates::Specific(Box::new(specific_templates)),
        }
    }

    fn handler_fixture(pool: &PgPool) -> AlertHandler {
        AlertHandler::new(
            Arc::new(NotificationDispatcher::new(
                dummy_config_fixture(),
                AlertDb::new(pool.clone()),
            )),
            pool.clone(),
        )
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures(
            "../../fixtures/new_user_registration.sql",
            "../../fixtures/node_alerts_active.sql",
        )
    )]
    #[ignore]
    async fn test_filter_duplicate_alerts(pool: PgPool) {
        let handler = handler_fixture(&pool);
        let machine_id = Uuid::parse_str("dcbf22c7-9d96-47ac-bf06-62d6544e440d").unwrap();
        let node_name = "test_node_123123".to_string();
        let alert_type_1 = Alert::Custom {
            node_name: node_name.clone(),
            node_type: NodeType::EigenDA.to_string(),
            extra_data: serde_json::Value::String("runtime_alert_fixture_1".to_string()),
        };

        let new_alert_1 = NewAlert::new(machine_id, alert_type_1, node_name.clone());

        let alert_type_2 = Alert::Custom {
            node_name: node_name.clone(),
            node_type: NodeType::EigenDA.to_string(),
            extra_data: serde_json::Value::String("runtime_alert_fixture_2".to_string()),
        };
        let new_alert_2 = NewAlert::new(machine_id, alert_type_2.clone(), node_name);

        NodeActiveAlert::insert_one(&pool, &new_alert_1).await.unwrap();

        let alerts = vec![new_alert_1, new_alert_2];

        let filtered_alerts = handler.filter_duplicate_alerts(alerts).await.unwrap();

        assert_eq!(filtered_alerts.len(), 0);
        // assert_eq!(filtered_alerts[0].alert_type, alert_type_2);
    }
}
