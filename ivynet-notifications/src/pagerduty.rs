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
pub enum Severity {
    Critical,
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => f.write_str("critical"),
            Self::Error => f.write_str("error"),
            Self::Warning => f.write_str("warning"),
            Self::Info => f.write_str("info"),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub enum Action {
    Trigger,
    Acknowledge,
    Resolve,
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trigger => f.write_str("trigger"),
            Self::Acknowledge => f.write_str("acknowledge"),
            Self::Resolve => f.write_str("resolve"),
        }
    }
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
        NotificationType::NodeNotRunning(avs) |
        NotificationType::NoChainInfo(avs) |
        NotificationType::NoMetrics(avs) |
        NotificationType::NoOperatorId(avs) => Some(avs.to_owned()),
        NotificationType::LowPerformaceScore { avs, performance: _ } => Some(avs.to_owned()),
        NotificationType::NeedsUpdate { avs, current_version: _, recommended_version: _ } => {
            Some(avs.to_owned())
        }
        _ => None,
    }
}
