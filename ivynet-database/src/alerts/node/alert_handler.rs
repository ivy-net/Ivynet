use std::{collections::HashMap, sync::Arc};

use ivynet_alerts::{Alert, SendState};
use ivynet_error::ethers::types::Chain;
use ivynet_grpc::messages::NodeDataV2;
use ivynet_node_type::NodeType;
use ivynet_notifications::{Channel, NotificationDispatcher, NotificationDispatcherError};

use async_trait::async_trait;
use sqlx::{types::Uuid, PgPool};

use crate::{
    alerts::{
        alert_db::AlertDb,
        alert_handler::{ActiveAlert, AlertHandler, NewAlert},
    },
    avs_version::{NodeTypeId, VersionData},
    data::{
        avs_version::{extract_semver, VersionType},
        node_data::UpdateStatus,
    },
    error::DatabaseError,
    Avs, DbAvsVersionData, Machine,
};

use super::alerts_active::{NewNodeAlert, NodeActiveAlert};

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
pub enum NodeAlertError {
    #[error(transparent)]
    DbError(#[from] DatabaseError),
    #[error(transparent)]
    NotificationError(#[from] NotificationDispatcherError),
    #[error(transparent)]
    SqxlError(#[from] sqlx::Error),
}

impl NewAlert for NewNodeAlert {
    fn get_id(&self) -> Uuid {
        self.id
    }

    fn get_alert_type(&self) -> Alert {
        self.alert_type.clone()
    }

    fn set_send_state(&mut self, channel: &Channel, state: SendState) {
        match channel {
            Channel::Telegram(_) => self.telegram_send = state,
            Channel::Email(_) => self.sendgrid_send = state,
            Channel::PagerDuty(_) => self.pagerduty_send = state,
        }
    }
}

impl ActiveAlert for NodeActiveAlert {
    fn get_id(&self) -> Uuid {
        self.alert_id
    }

    fn get_alert_type(&self) -> Alert {
        self.alert_type.clone()
    }
}
#[derive(Clone)]
pub struct NodeAlertHandler {
    pub dispatcher: Arc<NotificationDispatcher<AlertDb>>,
    db_executor: PgPool,
}

impl NodeAlertHandler {
    pub fn new(dispatcher: Arc<NotificationDispatcher<AlertDb>>, db_executor: PgPool) -> Self {
        Self { dispatcher, db_executor }
    }

    pub async fn handle_node_data_alerts(
        &self,
        node_data: NodeDataV2,
        machine_id: Uuid,
    ) -> Result<(), NodeAlertError> {
        let organization_id = Machine::get_organization_id(&self.db_executor, machine_id).await?;

        let new_alerts = extract_node_data_alerts(&self.db_executor, machine_id, &node_data)
            .await
            .into_iter()
            .map(|alert| NewNodeAlert::new(machine_id, alert, node_data.name.clone()))
            .collect::<Vec<_>>();

        let existing_alerts =
            NodeActiveAlert::all_alerts_by_org(&self.db_executor, organization_id).await?;

        let mut filtered_new_alerts =
            self.filter_duplicate_alerts(new_alerts, existing_alerts).await?;

        self.send_notifications(&mut filtered_new_alerts, organization_id as u64, Some(machine_id))
            .await?;

        NodeActiveAlert::insert_many(&self.db_executor, &filtered_new_alerts).await?;

        // Resolve step
        run_machine_alert_resolution(&self.db_executor, machine_id).await?;

        Ok(())
    }
}

#[async_trait]
impl AlertHandler for NodeAlertHandler {
    type Error = NodeAlertError;
    type NewAlertType = NewNodeAlert;
    type ActiveAlertType = NodeActiveAlert;

    fn get_dispatcher(&self) -> &Arc<NotificationDispatcher<AlertDb>> {
        &self.dispatcher
    }

    fn get_db_pool(&self) -> &PgPool {
        &self.db_executor
    }

    async fn filter_duplicate_alerts(
        &self,
        incoming_alerts: Vec<NewNodeAlert>,
        existing_alerts: Vec<NodeActiveAlert>,
    ) -> Result<Vec<NewNodeAlert>, NodeAlertError> {
        let existing_ids = existing_alerts.iter().map(|alert| alert.alert_id).collect::<Vec<_>>();

        let new_filtered_alerts = incoming_alerts
            .into_iter()
            .filter(|alert| !existing_ids.contains(&alert.id))
            .collect::<Vec<_>>();

        Ok(new_filtered_alerts)
    }
}

/// Fetch the latest AVS data for a machine. Compare alerts derived from the AVS data with the
/// existing alerts in the database. Resolve any alerts that are no longer present in the AVS data.
pub async fn run_machine_alert_resolution(
    pool: &PgPool,
    machine_id: Uuid,
) -> Result<(), NodeAlertError> {
    let avses = Avs::get_machines_avs_list(pool, machine_id).await?;
    let alerts = build_alerts_from_avses(pool, avses).await?;
    resolve_machine_alerts(pool, alerts, machine_id).await?;
    Ok(())
}

/// Fetch the latest AVS data for an organization. Compare alerts derived from the AVS data with
/// the existing alerts in the database. Resolve any alerts that are no longer present in the AVS
/// data.
pub async fn run_org_alert_resolution(pool: &PgPool, org_id: i64) -> Result<(), NodeAlertError> {
    let avses = Avs::get_org_avs_list(pool, org_id).await?;
    let alerts = build_alerts_from_avses(pool, avses).await?;
    resolve_org_alerts(pool, alerts, org_id).await?;
    Ok(())
}

async fn build_alerts_from_avses(
    pool: &PgPool,
    avses: Vec<Avs>,
) -> Result<Vec<NewNodeAlert>, DatabaseError> {
    let mut alerts = vec![];
    let version_map = DbAvsVersionData::get_all_avs_version(pool).await?;

    for avs in avses {
        let derived_alerts = alerts_from_avs(&avs, &version_map);
        let new_alerts = derived_alerts
            .into_iter()
            .map(|alert| NewNodeAlert::new(avs.machine_id, alert, avs.avs_name.clone()))
            .collect::<Vec<_>>();
        alerts.extend(new_alerts);
    }
    Ok(alerts)
}

pub fn alerts_from_avs(avs: &Avs, version_map: &HashMap<NodeTypeId, VersionData>) -> Vec<Alert> {
    let mut alerts = vec![];

    if !avs.active_set {
        alerts.push(Alert::UnregisteredFromActiveSet {
            node_name: avs.avs_name.clone(),
            node_type: avs.avs_type.to_string(),
            operator: avs.operator_address.unwrap_or_default(),
        });
    }

    if let Some(datetime) = avs.updated_at {
        let now = chrono::Utc::now().naive_utc();
        if now.signed_duration_since(datetime).num_minutes() > IDLE_MINUTES_THRESHOLD {
            alerts.push(Alert::NodeNotResponding {
                node_name: avs.avs_name.clone(),
                node_type: avs.avs_type.to_string(),
            });
            if avs.active_set && avs.operator_address.is_some() {
                alerts.push(Alert::ActiveSetNoDeployment {
                    node_name: avs.avs_name.clone(),
                    node_type: avs.avs_type.to_string(),
                    operator: avs.operator_address.expect("UNENTERABLE"),
                });
            }
        }
    }

    if !avs.metrics_alive {
        alerts.push(Alert::NoMetrics {
            node_name: avs.avs_name.clone(),
            node_type: avs.avs_type.to_string(),
        });
    }

    if !avs.node_running {
        alerts.push(Alert::NodeNotRunning {
            node_name: avs.avs_name.clone(),
            node_type: avs.avs_type.to_string(),
        });
    }

    if avs.chain.is_none() {
        alerts.push(Alert::NoChainInfo {
            node_name: avs.avs_name.clone(),
            node_type: avs.avs_type.to_string(),
        });
    } else if let Some(chain) = avs.chain {
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
            if update_status == UpdateStatus::Outdated || update_status == UpdateStatus::Updateable
            {
                alerts.push(Alert::NodeNeedsUpdate {
                    node_name: avs.avs_name.clone(),
                    node_type: avs.avs_type.to_string(),
                    current_version: avs.avs_version.clone(),
                    recommended_version,
                });
            }
        }
    }

    if avs.operator_address.is_none() {
        alerts.push(Alert::NoOperatorId {
            node_name: avs.avs_name.clone(),
            node_type: avs.avs_type.to_string(),
        });
    }

    alerts
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

    if !avs.active_set {
        alerts.push(Alert::UnregisteredFromActiveSet {
            node_name: avs.avs_name,
            node_type: avs.avs_type.to_string(),
            operator: avs.operator_address.unwrap_or_default(),
        });
    }

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
                    alerts.push(Alert::NodeNeedsUpdate {
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

pub async fn resolve_org_alerts(
    pool: &PgPool,
    alerts: Vec<NewNodeAlert>,
    org_id: i64,
) -> Result<(), DatabaseError> {
    let db_alerts = NodeActiveAlert::all_alerts_by_org(pool, org_id).await?;

    // Filter existing alerts, removing any that are not in the incoming list
    let to_resolve = db_alerts
        .into_iter()
        .filter(|alert| !alerts.iter().any(|new_alert| new_alert.get_id() == alert.alert_id))
        .collect::<Vec<_>>();

    for alert in to_resolve {
        NodeActiveAlert::resolve_alert(pool, alert.alert_id).await?;
    }

    Ok(())
}

pub async fn resolve_machine_alerts(
    pool: &PgPool,
    alerts: Vec<NewNodeAlert>,
    machine_id: Uuid,
) -> Result<(), DatabaseError> {
    let db_alerts = NodeActiveAlert::all_alerts_by_machine(pool, machine_id).await?;

    // Filter existing alerts, removing any that are not in the incoming list
    let to_resolve = db_alerts
        .into_iter()
        .filter(|alert| !alerts.iter().any(|new_alert| new_alert.get_id() == alert.alert_id))
        .collect::<Vec<_>>();

    for alert in to_resolve {
        NodeActiveAlert::resolve_alert(pool, alert.alert_id).await?;
    }

    Ok(())
}

pub async fn resolve_node_alerts(
    pool: &PgPool,
    alerts: Vec<NewNodeAlert>,
    nodes: Vec<Avs>,
) -> Result<(), DatabaseError> {
    let db_alerts = NodeActiveAlert::get_by_avs_list(pool, &nodes).await?;

    // Filter existing alerts, removing any that are not in the incoming list
    let to_resolve = db_alerts
        .into_iter()
        .filter(|alert| !alerts.iter().any(|new_alert| new_alert.get_id() == alert.alert_id))
        .collect::<Vec<_>>();

    for alert in to_resolve {
        NodeActiveAlert::resolve_alert(pool, alert.alert_id).await?;
    }

    Ok(())
}

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

    fn handler_fixture(pool: &PgPool) -> NodeAlertHandler {
        NodeAlertHandler::new(
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
            "../../../fixtures/new_user_registration.sql",
            "../../../fixtures/node_alerts_active.sql",
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

        let new_alert_1 = NewNodeAlert::new(machine_id, alert_type_1, node_name.clone());

        let alert_type_2 = Alert::Custom {
            node_name: node_name.clone(),
            node_type: NodeType::EigenDA.to_string(),
            extra_data: serde_json::Value::String("runtime_alert_fixture_2".to_string()),
        };
        let new_alert_2 = NewNodeAlert::new(machine_id, alert_type_2.clone(), node_name);

        NodeActiveAlert::insert_one(&pool, &new_alert_1).await.unwrap();

        let alerts = vec![new_alert_1, new_alert_2];

        let existing_alerts = NodeActiveAlert::all_alerts_by_org(&pool, 1).await.unwrap();

        let filtered_alerts =
            handler.filter_duplicate_alerts(alerts, existing_alerts).await.unwrap();

        assert_eq!(filtered_alerts.len(), 1);
        assert_eq!(filtered_alerts[0].alert_type, alert_type_2);
    }
}
