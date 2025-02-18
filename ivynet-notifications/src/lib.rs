use std::collections::HashSet;

use ethers::types::H160;
use pagerduty::PagerDutySender;
use sendgrid::EmailSender;
use serde::{Deserialize, Serialize};
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

/// Integer flags used primarily for identification with the From<> trait, as the actual values are
/// unusable for `as i32` conversions as they have associated fields. This apparently also
/// suppresses null-pointer optimization.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum NotificationType {
    /// Custom message
    Custom(String) = 0,
    /// User ETH address
    UnregisteredFromActiveSet {
        avs: String,
        address: H160,
    } = 1,
    MachineNotResponding = 2,
    /// Node name
    NodeNotRunning(String) = 3,
    /// Node name
    NoChainInfo(String) = 4,
    /// Node name
    NoMetrics(String) = 5,
    /// Node name
    NoOperatorId(String) = 6,
    HardwareResourceUsage {
        resource: String,
        percent: u16,
    } = 7,
    LowPerformaceScore {
        avs: String,
        performance: u16,
    } = 8,
    NeedsUpdate {
        avs: String,
        current_version: String,
        recommended_version: String,
    } = 9,
}

impl From<NotificationType> for i32 {
    fn from(notification_type: NotificationType) -> Self {
        match notification_type {
            NotificationType::Custom(_) => 0,
            NotificationType::UnregisteredFromActiveSet { .. } => 1,
            NotificationType::MachineNotResponding => 2,
            NotificationType::NodeNotRunning(_) => 3,
            NotificationType::NoChainInfo(_) => 4,
            NotificationType::NoMetrics(_) => 5,
            NotificationType::NoOperatorId(_) => 6,
            NotificationType::HardwareResourceUsage { .. } => 7,
            NotificationType::LowPerformaceScore { .. } => 8,
            NotificationType::NeedsUpdate { .. } => 9,
        }
    }
}

impl From<NotificationType> for i64 {
    fn from(notification_type: NotificationType) -> Self {
        i32::from(notification_type) as i64
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
    async fn get_emails_for_organization(&self, organization_id: u64) -> HashSet<String>;
    async fn get_chats_for_organization(&self, organization_id: u64) -> HashSet<String>;
    async fn get_pd_integration_key_for_organization(&self, organization_id: u64)
        -> Option<String>;
}

impl<D: OrganizationDatabase> NotificationDispatcher<D> {
    pub fn new(config: NotificationConfig, db: D) -> Self {
        Self {
            telegram: TelegramBot::<D>::new(config.telegram_token, db.clone()),
            email_sender: EmailSender::new(&config, db.clone()),
            pagerduty: PagerDutySender::new(db),
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
