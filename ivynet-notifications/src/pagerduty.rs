use std::collections::HashSet;

use crate::{Notification, OrganizationDatabase};
use chrono::{DateTime, Utc};
use ivynet_alerts::Alert;
use serde::Serialize;
use uuid::Uuid;

type NotificationType = Alert;

const PAGER_DUTY_Q_URL: &str = "https://events.pagerduty.com/v2/enqueue";

#[derive(thiserror::Error, Debug)]
pub enum PagerDutySenderError {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Trigger,
    Acknowledge,
    Resolve,
}

/// Payload of the event that is being sent
#[derive(Clone, Debug, Serialize)]
pub struct Payload {
    pub severity: Severity,
    pub source: String,
    pub summary: String,
    pub timestamp: DateTime<Utc>,
    pub component: Option<String>,
}

/// Struct of the event to send to PagerDuty service
#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub routing_key: String,
    pub event_action: Action,
    pub dedup_key: Uuid,
    pub client: Option<String>,
    pub payload: Payload,
}

impl From<Notification> for Event {
    fn from(value: Notification) -> Self {
        Self {
            routing_key: "".to_owned(),
            event_action: if value.resolved { Action::Resolve } else { Action::Trigger },
            dedup_key: value.id,
            client: avs_if_any(&value),
            payload: Payload {
                severity: Severity::Error, // TODO: Maybe we should vary it depending on the
                // notification type?
                source: "IvyNet".to_owned(),
                summary: value.to_pagerduty_message(),
                timestamp: chrono::Local::now().into(),
                component: Some(format!("{:?}", value.machine_id)),
            },
        }
    }
}

fn avs_if_any(notification: &Notification) -> Option<String> {
    match &notification.alert {
        NotificationType::NodeNotRunning { node_name: name, .. } |
        NotificationType::NoChainInfo { node_name: name, .. } |
        NotificationType::NoMetrics { node_name: name, .. } |
        NotificationType::NoOperatorId { node_name: name, .. } |
        NotificationType::LowPerformanceScore { node_name: name, .. } |
        NotificationType::NodeNeedsUpdate { node_name: name, .. } |
        NotificationType::Custom { node_name: name, .. } |
        NotificationType::ActiveSetNoDeployment { node_name: name, .. } |
        NotificationType::UnregisteredFromActiveSet { node_name: name, .. } |
        NotificationType::NodeNotResponding { node_name: name, .. } |
        NotificationType::NewEigenAvs { name, .. } |
        NotificationType::UpdatedEigenAvs { name, .. } => Some(name.to_owned()),
        NotificationType::HardwareResourceUsage { .. } => None,
        NotificationType::IdleMachine { .. } => None,
        NotificationType::ClientUpdateRequired { .. } => None,
        // TODO: This is somewhat redundant with the `impl` methods for constructing notifications.
        // Should be standardized.
        NotificationType::NoClientHeartbeat => None,
        NotificationType::NoMachineHeartbeat => None,
        NotificationType::NoNodeHeartbeat => None,
    }
}

pub struct PagerDutySender<D: OrganizationDatabase> {
    pub client: reqwest::Client,
    pub db: D,
}

impl<D: OrganizationDatabase> PagerDutySender<D> {
    pub fn new(db: D) -> Self {
        Self { client: reqwest::Client::new(), db }
    }

    pub async fn notify(
        &self,
        notification: impl PagerDutySend,
        keys: &HashSet<String>,
    ) -> Result<(), PagerDutySenderError> {
        for key in keys {
            let mut event: Event = notification.clone().into();
            event.routing_key = key.clone();
            self.send(event).await?;
        }
        Ok(())
    }

    async fn send(&self, event: Event) -> Result<(), PagerDutySenderError> {
        self.client.post(PAGER_DUTY_Q_URL).json(&event).send().await?;

        Ok(())
    }
}

pub trait PagerDutySend: Into<Event> + Clone {
    fn to_pagerduty_message(&self) -> String;
}

impl PagerDutySend for Notification {
    fn to_pagerduty_message(&self) -> String {
        match &self.alert {
            NotificationType::UnregisteredFromActiveSet { node_name, operator, .. } => {
                format!("Address {operator:?} has been removed from the active set for {node_name}")
            }
            NotificationType::Custom { extra_data, .. } => format!("ERROR: {extra_data}"),
            NotificationType::NodeNotRunning { node_name, .. } => {
                format!("AVS {node_name} is not running on {}", self.machine_id.unwrap_or_default())
            }
            NotificationType::NoChainInfo { node_name, .. } => {
                format!("No information on chain for avs {node_name}")
            }
            NotificationType::NoMetrics { node_name, .. } => {
                format!("No metrics reported from avs {node_name}")
            }
            NotificationType::NoOperatorId { node_name, .. } => {
                format!("No operator configured for {node_name}")
            }
            NotificationType::HardwareResourceUsage { resource, .. } => {
                format!(
                    "Machine {} is maxing out hardware resources: {}",
                    self.machine_id.unwrap_or_default(),
                    resource
                )
            }
            NotificationType::LowPerformanceScore { node_name, performance, .. } => {
                format!("AVS {node_name} has droped in performance to {performance}")
            }
            NotificationType::NodeNeedsUpdate {
                node_name,
                current_version,
                recommended_version,
                ..
            } => {
                format!(
                    "AVS {node_name} needs update from {current_version} to {recommended_version}"
                )
            }
            NotificationType::ActiveSetNoDeployment { node_name, operator, .. } => {
                format!("The validator {operator} for {node_name} is in the active set, but the node is either not deployed or not responding")
            }
            NotificationType::NodeNotResponding { node_name, .. } => {
                format!("The node {node_name} is not responding")
            }
            NotificationType::NewEigenAvs {
                address,
                name,
                metadata_uri,
                description,
                website,
                twitter,
                ..
            } => {
                format!("New EigenLayer AVS: {name} has been detected at {:?} with metadata URI {metadata_uri}. \n Website: {website} \n Twitter: {twitter} \n Description: {description}", address)
            }
            NotificationType::UpdatedEigenAvs {
                address,
                name,
                metadata_uri,
                website,
                twitter,
                ..
            } => {
                format!("Updated EigenLayer AVS: {name} has updated their metadata or address to {:?} with metadata URI {metadata_uri}. \n Website: {website} \n Twitter: {twitter}", address)
            }
            // TODO: As above, currently unused, only here for compiler completeness.
            NotificationType::NoClientHeartbeat => "No heartbeat from client".to_string(),
            NotificationType::NoMachineHeartbeat => "No heartbeat from machine".to_string(),
            NotificationType::NoNodeHeartbeat => "No heartbeat from node".to_string(),
            NotificationType::IdleMachine { .. } => {
                format!("Machine {} has no running nodes", self.machine_id.unwrap_or_default())
            }
            NotificationType::ClientUpdateRequired { .. } => {
                format!(
                    "Machine {} needs an update to the Ivynet client",
                    self.machine_id.unwrap_or_default()
                )
            }
        }
    }
}

#[cfg(test)]
mod pagerduty_live_test {
    use std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    };

    use serde_json;
    use tokio::sync::Mutex;

    use crate::{RegistrationResult, UnregistrationResult};

    use super::*;

    static MOCK_ORGANIZATION_ID: u64 = 1;

    /// Set your integration key to perform live test
    static MOCK_INTEGRATION_ID: &str = "";
    #[derive(Debug)]
    struct MockDbBackend {
        chats: HashMap<u64, HashSet<String>>,
    }

    impl MockDbBackend {
        fn new() -> Self {
            Self { chats: HashMap::new() }
        }
        fn add_chat(&mut self, organization_id: u64, chat_id: &str) -> RegistrationResult {
            if self.chats.values().any(|chats| chats.contains(chat_id)) {
                RegistrationResult::AlreadyRegistered
            } else {
                self.chats.entry(organization_id).or_default().insert(chat_id.to_string());
                RegistrationResult::Success
            }
        }
        fn remove_chat(&mut self, chat_id: &str) -> UnregistrationResult {
            for chats in self.chats.values_mut() {
                if chats.remove(chat_id) {
                    return UnregistrationResult::Success;
                }
            }
            UnregistrationResult::ChatNotRegistered
        }
        fn chats_for(&self, organization_id: u64) -> HashSet<String> {
            self.chats.get(&organization_id).cloned().unwrap_or_default()
        }
    }

    #[derive(Clone, Debug)]
    struct MockDb(Arc<Mutex<MockDbBackend>>);

    impl MockDb {
        fn new() -> Self {
            Self(Arc::new(Mutex::new(MockDbBackend::new())))
        }
    }

    #[async_trait::async_trait]
    impl OrganizationDatabase for MockDb {
        async fn register_chat(
            &self,
            chat_id: &str,
            _email: &str,
            _password: &str,
        ) -> RegistrationResult {
            let mut db = self.0.lock().await;
            db.add_chat(MOCK_ORGANIZATION_ID, chat_id)
        }

        async fn unregister_chat(&self, chat_id: &str) -> UnregistrationResult {
            let mut db = self.0.lock().await;
            db.remove_chat(chat_id)
        }

        async fn get_emails_for_organization(&self, _organization_id: u64) -> HashSet<String> {
            HashSet::new()
        }

        async fn get_chats_for_organization(&self, organization_id: u64) -> HashSet<String> {
            let db = self.0.lock().await;
            db.chats_for(organization_id)
        }

        async fn get_pd_integration_keys_for_organization(
            &self,
            _organization_id: u64,
        ) -> HashSet<String> {
            if MOCK_INTEGRATION_ID.is_empty() {
                HashSet::new()
            } else {
                let mut set = HashSet::new();
                set.insert(MOCK_INTEGRATION_ID.to_string());
                set
            }
        }
    }

    #[tokio::test]
    async fn test_raising_event() {
        let db = MockDb::new();
        let keys = db.get_pd_integration_keys_for_organization(MOCK_ORGANIZATION_ID).await;

        let pagerduty = PagerDutySender::new(db);
        let mut test_event = Notification {
            id: Uuid::new_v4(),
            organization: MOCK_ORGANIZATION_ID,
            machine_id: Some(Uuid::new_v4()),
            alert: Alert::Custom {
                node_name: "test-node".to_string(),
                node_type: "test-type".to_string(),
                extra_data: serde_json::json!({
                    "message": "We are testing sending events"
                }),
            },
            resolved: false,
        };

        assert!(pagerduty.notify(test_event.clone(), &keys).await.is_ok());
        test_event.resolved = true;
        assert!(pagerduty.notify(test_event.clone(), &keys).await.is_ok());
    }
}
