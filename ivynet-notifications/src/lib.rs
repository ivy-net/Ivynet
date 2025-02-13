use std::collections::HashSet;

use ethers::types::H160;
use pagerduty::PagerDutySender;
use sendgrid::EmailSender;
use serde::de::value::Error as SerdeError;
use serde::de::Error;
use serde::Serialize;
use telegram::TelegramBot;
use uuid::Uuid;

pub mod pagerduty;
pub mod sendgrid;
pub mod telegram;

#[derive(thiserror::Error, Debug)]
pub enum NotificationDispatcherError {
    #[error(transparent)]
    BotError(#[from] telegram::BotError),

    #[error(transparent)]
    EmailSenderError(#[from] sendgrid::EmailSenderError),

    #[error(transparent)]
    PagerDutyError(#[from] pagerduty::PagerDutySenderError),
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: Uuid,
    pub organization: u64,
    pub machine_id: Uuid,
    pub notification_type: NotificationType,
    pub resolved: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[repr(i64)]
pub enum NotificationType {
    Custom(String) = 0,
    UnregisteredFromActiveSet(H160) = 1,
    MachineNotResponding = 2,
    NodeNotRunning(String) = 3,
    NoChainInfo(String) = 4,
    NoMetrics(String) = 5,
    NoOperatorId(String) = 6,
    HardwareResourceUsage { resource: String, percent: u16 } = 7,
    LowPerformaceScore { avs: String, performance: u16 } = 8,
    NeedsUpdate { avs: String, current_version: String, recommended_version: String } = 9,
}

/// Intended for use with the DbActiveAlert type. Decodes a tuple of (i64, serde_json::Value) into
/// a NotificationType.
impl TryFrom<(i64, Option<serde_json::Value>)> for NotificationType {
    type Error = SerdeError;

    fn try_from(value: (i64, Option<serde_json::Value>)) -> Result<Self, Self::Error> {
        let (notification_type, custom_data) = value;
        match notification_type {
            0 => Ok(NotificationType::Custom(
                custom_data
                    .ok_or_else(|| Error::custom("Missing custom data"))?
                    .as_str()
                    .ok_or_else(|| Error::custom("Custom data is not a string"))?
                    .to_string(),
            )),
            1 => Ok(NotificationType::UnregisteredFromActiveSet(
                custom_data
                    .ok_or_else(|| Error::custom("Missing custom data"))?
                    .as_str()
                    .ok_or_else(|| Error::custom("Custom data is not a string"))?
                    .parse()
                    .map_err(|_| Error::custom("Could not parse H160"))?,
            )),
            2 => Ok(NotificationType::MachineNotResponding),
            3 => Ok(NotificationType::NodeNotRunning(
                custom_data
                    .ok_or_else(|| Error::custom("Missing custom data"))?
                    .as_str()
                    .ok_or_else(|| Error::custom("Custom data is not a string"))?
                    .to_string(),
            )),
            4 => Ok(NotificationType::NoChainInfo(
                custom_data
                    .ok_or_else(|| Error::custom("Missing custom data"))?
                    .as_str()
                    .ok_or_else(|| Error::custom("Custom data is not a string"))?
                    .to_string(),
            )),
            5 => Ok(NotificationType::NoMetrics(
                custom_data
                    .ok_or_else(|| Error::custom("Missing custom data"))?
                    .as_str()
                    .ok_or_else(|| Error::custom("Custom data is not a string"))?
                    .to_string(),
            )),
            6 => Ok(NotificationType::NoOperatorId(
                custom_data
                    .ok_or_else(|| Error::custom("Missing custom data"))?
                    .as_str()
                    .ok_or_else(|| Error::custom("Custom data is not a string"))?
                    .to_string(),
            )),
            7 => {
                let custom_data =
                    custom_data.ok_or_else(|| Error::custom("Missing custom data"))?;
                Ok(NotificationType::HardwareResourceUsage {
                    resource: custom_data
                        .get("resource")
                        .ok_or_else(|| Error::custom("Missing resource field"))?
                        .as_str()
                        .ok_or_else(|| Error::custom("Resource field is not a string"))?
                        .to_string(),
                    percent: custom_data
                        .get("percent")
                        .ok_or_else(|| Error::custom("Missing percent field"))?
                        .as_u64()
                        .ok_or_else(|| Error::custom("Percent field is not a number"))?
                        .try_into()
                        .map_err(|_| Error::custom("Percent field is not a valid u16"))?,
                })
            }
            8 => {
                let custom_data =
                    custom_data.ok_or_else(|| Error::custom("Missing custom data"))?;
                Ok(NotificationType::LowPerformaceScore {
                    avs: custom_data
                        .get("avs")
                        .ok_or_else(|| Error::custom("Missing avs field"))?
                        .as_str()
                        .ok_or_else(|| Error::custom("Avs field is not a string"))?
                        .to_string(),
                    performance: custom_data
                        .get("performance")
                        .ok_or_else(|| Error::custom("Missing performance field"))?
                        .as_u64()
                        .ok_or_else(|| Error::custom("Performance field is not a number"))?
                        .try_into()
                        .map_err(|_| Error::custom("Performance field is not a valid u16"))?,
                })
            }
            9 => {
                let custom_data =
                    custom_data.ok_or_else(|| Error::custom("Missing custom data"))?;
                Ok(NotificationType::NeedsUpdate {
                    avs: custom_data
                        .get("avs")
                        .ok_or_else(|| Error::custom("Missing avs field"))?
                        .as_str()
                        .ok_or_else(|| Error::custom("Avs field is not a string"))?
                        .to_string(),
                    current_version: custom_data
                        .get("current_version")
                        .ok_or_else(|| Error::custom("Missing current_version field"))?
                        .as_str()
                        .ok_or_else(|| Error::custom("Current_version field is not a string"))?
                        .to_string(),
                    recommended_version: custom_data
                        .get("recommended_version")
                        .ok_or_else(|| Error::custom("Missing recommended_version field"))?
                        .as_str()
                        .ok_or_else(|| Error::custom("Recommended_version field is not a string"))?
                        .to_string(),
                })
            }
            _ => Err(Error::custom("Unknown notification type")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Channel {
    Telegram,
    Email,
    PagerDuty,
}

#[derive(Clone, Debug)]
pub struct SendgridTemplates<'a> {
    pub custom: &'a str,
    pub unreg_active_set: &'a str,
    pub machine_not_responding: &'a str,
    pub node_not_running: &'a str,
    pub no_chain_info: &'a str,
    pub no_metrics: &'a str,
    pub no_operator: &'a str,
    pub hw_res_usage: &'a str,
    pub low_perf: &'a str,
    pub needs_update: &'a str,
}

#[derive(Clone, Debug)]
pub struct NotificationConfig<'a> {
    pub pagerduty_token: &'a str,
    pub telegram_token: &'a str,
    pub sendgrid_key: &'a str,
    pub sendgrid_from: &'a str,
    pub sendgrid_templates: SendgridTemplates<'a>,
}

pub struct NotificationDispatcher<D: OrganizationDatabase> {
    pub telegram: TelegramBot<D>,
    pub email_sender: EmailSender<D>,
    pub pagerduty: PagerDutySender<D>,
}

#[async_trait::async_trait]
pub trait OrganizationDatabase: Send + Sync + Clone + 'static {
    async fn register_chat(&self, chat_id: &str, email: &str, password: &str) -> bool;
    async fn unregister_chat(&self, chat_id: &str) -> bool;
    async fn get_emails_for_organization(&self, organization_id: u64) -> Vec<String>;
    async fn get_chats_for_organization(&self, organization_id: u64) -> Vec<String>;
    async fn get_pd_integration_key_for_organization(&self, organization_id: u64)
        -> Option<String>;
}

impl<D: OrganizationDatabase> NotificationDispatcher<D> {
    pub fn new(config: NotificationConfig, db: D) -> Self {
        Self {
            telegram: TelegramBot::<D>::new(config.telegram_token, db.clone()),
            email_sender: EmailSender::new(&config, db.clone()),
            pagerduty: PagerDutySender::new(config.pagerduty_token, db),
        }
    }

    pub async fn serve(&self) -> Result<(), NotificationDispatcherError> {
        self.telegram.serve().await?;
        Ok(())
    }

    pub async fn notify(
        &self,
        notification: Notification,
        channels: HashSet<Channel>,
    ) -> Result<(), NotificationDispatcherError> {
        for channel in channels {
            match channel {
                Channel::Email => self.email_sender.notify(notification.clone()).await?,
                Channel::Telegram => self.telegram.notify(notification.clone()).await?,
                Channel::PagerDuty => self.pagerduty.notify(notification.clone()).await?,
            }
        }
        Ok(())
    }
}
