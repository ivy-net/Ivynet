use std::collections::HashSet;

use ethers::types::H160;
use pagerduty::PagerDutySender;
use sendgrid::EmailSender;
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
    pub organization: Uuid,
    pub machine_id: H160,
    pub notification_type: NotificationType,
    pub resolved: bool,
}

#[derive(Debug, Clone)]
pub enum NotificationType {
    Custom(String),
    UnregisteredFromActiveSet(H160),
    CrashedNode,
    NodeNotRunning(String),
    NoChainInfo(String),
    NoMetrics(String),
    NoOperatorId(String),
    HardwareResourceUsage { resource: String, percent: u16 },
    LowPerformaceScore { avs: String, performance: u16 },
    NeedsUpdate { avs: String, current_version: String, recommended_version: String },
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
    pub crashed_node: &'a str,
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
    async fn get_emails_for_organization(&self, organization_id: Uuid) -> Vec<String>;
    async fn get_chats_for_organization(&self, organization_id: Uuid) -> Vec<String>;
    async fn get_pd_integration_key_for_organization(
        &self,
        organization_id: Uuid,
    ) -> Option<String>;
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
