use std::{collections::HashSet, fmt::Debug};

use ivynet_alerts::Alert;
use pagerduty::{PagerDutySend, PagerDutySender};
use sendgrid::{EmailSender, SendgridSend};
use telegram::{TelegramBot, TelegramSend};
use uuid::Uuid;

pub mod pagerduty;
pub mod sendgrid;
pub mod telegram;

pub trait NotificationSend: PagerDutySend + SendgridSend + TelegramSend + Debug {}

#[derive(thiserror::Error, Debug)]
pub enum NotificationDispatcherError {
    #[error(transparent)]
    BotError(#[from] telegram::BotError),

    #[error(transparent)]
    EmailSenderError(#[from] sendgrid::EmailSenderError),

    #[error(transparent)]
    PagerDutyError(#[from] pagerduty::PagerDutySenderError),

    #[error("Database error")]
    DatabaseError,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: Uuid,
    pub organization: u64,
    pub machine_id: Option<Uuid>,
    pub alert: Alert,
    pub resolved: bool,
}

impl NotificationSend for Notification {}

#[derive(Clone, Debug)]
pub enum SendgridTemplates {
    Generic(String),
    Specific(Box<SendgridSpecificTemplates>),
}

#[derive(Clone, Debug)]
pub struct SendgridSpecificTemplates {
    // Node Data Alerts
    pub custom: String,
    pub unreg_active_set: String,
    pub machine_not_responding: String,
    pub node_not_running: String,
    pub no_chain_info: String,
    pub no_metrics: String,
    pub no_operator: String,
    pub hw_res_usage: String,
    pub low_perf: String,
    pub needs_update: String,

    //Event Data Alerts
    pub new_eigen_avs: String,
    pub updated_eigen_avs: String,
}

#[derive(Clone, Debug)]
pub struct NotificationConfig {
    pub telegram_token: String,
    pub sendgrid_key: String,
    pub sendgrid_from: String,
    pub sendgrid_templates: SendgridTemplates,
}

pub struct NotificationDispatcher<D: OrganizationDatabase> {
    pub telegram: TelegramBot<D>,
    pub email_sender: EmailSender<D>,
    pub pagerduty: PagerDutySender<D>,
}

#[derive(Debug)]
pub enum RegistrationResult {
    Success,
    AlreadyRegistered,
    AuthenticationFailed,
    DatabaseError(String),
}

#[derive(Debug)]
pub enum UnregistrationResult {
    Success,
    ChatNotRegistered,
    DatabaseError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Channel {
    Telegram(HashSet<String>),
    Email(HashSet<String>),
    PagerDuty(HashSet<String>),
}

#[async_trait::async_trait]
pub trait OrganizationDatabase: Send + Sync + Clone + 'static {
    async fn register_chat(&self, chat_id: &str, email: &str, password: &str)
        -> RegistrationResult;
    async fn unregister_chat(&self, chat_id: &str) -> UnregistrationResult;
    async fn get_emails_for_organization(&self, organization_id: u64) -> HashSet<String>;
    async fn get_chats_for_organization(&self, organization_id: u64) -> HashSet<String>;
    async fn get_pd_integration_keys_for_organization(
        &self,
        organization_id: u64,
    ) -> HashSet<String>;
}

impl<D: OrganizationDatabase> NotificationDispatcher<D> {
    pub fn new(config: NotificationConfig, db: D) -> Self {
        Self {
            telegram: TelegramBot::<D>::new(&config.telegram_token, db.clone()),
            email_sender: EmailSender::new(&config, db.clone()),
            pagerduty: PagerDutySender::new(db),
        }
    }

    pub async fn serve(&self) -> Result<(), NotificationDispatcherError> {
        self.telegram.serve().await?;
        Ok(())
    }

    pub async fn notify_channel(
        &self,
        notification: impl NotificationSend,
        channel: &Channel,
    ) -> bool {
        tracing::debug!("notifying channel: {:#?}", channel);
        tracing::debug!("notification: {:#?}", notification);

        let result = match channel {
            Channel::Email(emails) => self
                .email_sender
                .notify(notification, emails)
                .await
                .map_err(NotificationDispatcherError::EmailSenderError),
            Channel::Telegram(chats) => self
                .telegram
                .notify(notification, chats)
                .await
                .map_err(NotificationDispatcherError::BotError),
            Channel::PagerDuty(keys) => self
                .pagerduty
                .notify(notification, keys)
                .await
                .map_err(NotificationDispatcherError::PagerDutyError),
        };

        tracing::debug!("result: {:#?}", result);

        result.is_ok()
    }

    pub async fn notify(
        &self,
        notification: impl NotificationSend,
        channels: Vec<Channel>,
    ) -> Result<(), NotificationDispatcherError> {
        for channel in channels {
            match channel {
                Channel::Email(emails) => {
                    self.email_sender.notify(notification.clone(), &emails).await?
                }
                Channel::Telegram(chats) => {
                    self.telegram.notify(notification.clone(), &chats).await?
                }
                Channel::PagerDuty(keys) => {
                    self.pagerduty.notify(notification.clone(), &keys).await?
                }
            }
        }
        Ok(())
    }
}
