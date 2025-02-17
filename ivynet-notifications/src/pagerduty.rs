use crate::{Notification, NotificationType, OrganizationDatabase};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

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
struct Payload {
    pub severity: Severity,
    pub source: String,
    pub summary: String,
    pub timestamp: DateTime<Utc>,
    pub component: Option<String>,
}

/// Struct of the event to send to PagerDuty service
#[derive(Clone, Debug, Serialize)]
struct Event {
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
                summary: message(&value),
                timestamp: chrono::Local::now().into(),
                component: Some(format!("{:?}", value.machine_id)),
            },
        }
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

    pub async fn notify(&self, notification: Notification) -> Result<(), PagerDutySenderError> {
        if let Some(integration_key) =
            self.db.get_pd_integration_key_for_organization(notification.organization).await
        {
            let mut event: Event = notification.into();
            event.routing_key = integration_key;
            self.send(event).await?;
        }
        Ok(())
    }

    async fn send(&self, event: Event) -> Result<(), PagerDutySenderError> {
        self.client.post(PAGER_DUTY_Q_URL).json(&event).send().await?;

        Ok(())
    }
}

fn message(notification: &Notification) -> String {
    match &notification.notification_type {
        NotificationType::UnregisteredFromActiveSet { avs, address } => {
            format!("Address {address:?} has been removed from the active set for {avs}")
        }
        NotificationType::MachineNotResponding => {
            format!("Machine '{:?}' has lost connection with our backend", notification.machine_id)
        }
        NotificationType::Custom(msg) => format!("ERROR: {msg}"),
        NotificationType::NodeNotRunning(avs) => {
            format!("AVS {avs} is not running on {}", notification.machine_id)
        }
        NotificationType::NoChainInfo(avs) => format!("No information on chain for avs {avs}"),
        NotificationType::NoMetrics(avs) => format!("No metrics reported from avs {avs}"),
        NotificationType::NoOperatorId(avs) => format!("No operator configured for {avs}"),
        NotificationType::HardwareResourceUsage { resource, percent } => {
            format!("Machine {} has used over {percent}% of {resource}", notification.machine_id)
        }
        NotificationType::LowPerformaceScore { avs, performance } => {
            format!("AVS {avs} has droped in performance to {performance}")
        }
        NotificationType::NeedsUpdate { avs, current_version, recommended_version } => {
            format!("AVS {avs} needs update from {current_version} to {recommended_version}")
        }
    }
}

fn avs_if_any(notification: &Notification) -> Option<String> {
    match &notification.notification_type {
        NotificationType::NodeNotRunning(avs)
        | NotificationType::NoChainInfo(avs)
        | NotificationType::NoMetrics(avs)
        | NotificationType::NoOperatorId(avs) => Some(avs.to_owned()),
        NotificationType::LowPerformaceScore { avs, performance: _ } => Some(avs.to_owned()),
        NotificationType::NeedsUpdate { avs, current_version: _, recommended_version: _ } => {
            Some(avs.to_owned())
        }
        _ => None,
    }
}

#[cfg(test)]
mod pagerduty_live_test {

    use std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    };

    use tokio::sync::Mutex;

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
        fn add_chat(&mut self, organization_id: u64, chat_id: &str) -> bool {
            self.chats.entry(organization_id).or_default().insert(chat_id.to_string());
            true
        }
        fn remove_chat(&mut self, chat_id: &str) -> bool {
            for chats in self.chats.values_mut() {
                if chats.remove(chat_id) {
                    return true;
                }
            }
            false
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
        async fn register_chat(&self, chat_id: &str, _email: &str, _password: &str) -> bool {
            let mut db = self.0.lock().await;
            db.add_chat(MOCK_ORGANIZATION_ID, chat_id)
        }

        async fn unregister_chat(&self, chat_id: &str) -> bool {
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

        async fn get_pd_integration_key_for_organization(
            &self,
            _organization_id: u64,
        ) -> Option<String> {
            if MOCK_INTEGRATION_ID.is_empty() {
                None
            } else {
                Some(MOCK_INTEGRATION_ID.to_string())
            }
        }
    }

    #[tokio::test]
    async fn test_raising_event() {
        let db = MockDb::new();

        let pagerduty = PagerDutySender::new(db);

        let mut test_event = Notification {
            id: Uuid::new_v4(),
            organization: MOCK_ORGANIZATION_ID,
            machine_id: Uuid::new_v4(),
            notification_type: NotificationType::Custom(
                "We are testing sending events".to_string(),
            ),
            resolved: false,
        };

        assert!(pagerduty.notify(test_event.clone()).await.is_ok());
        test_event.resolved = true;
        assert!(pagerduty.notify(test_event.clone()).await.is_ok());
    }
}
