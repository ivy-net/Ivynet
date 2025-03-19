use std::collections::HashSet;

use ivynet_alerts::{Alert, Channel};
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
    pub organization: u64,
    pub machine_id: Option<Uuid>,
    pub alert: Alert,
    pub resolved: bool,
}

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

#[async_trait::async_trait]
pub trait OrganizationDatabase: Send + Sync + Clone + 'static {
    async fn register_chat(&self, chat_id: &str, email: &str, password: &str)
        -> RegistrationResult;
    async fn unregister_chat(&self, chat_id: &str) -> UnregistrationResult;
    async fn get_emails_for_organization(&self, organization_id: u64) -> HashSet<String>;
    async fn get_chats_for_organization(&self, organization_id: u64) -> HashSet<String>;
    async fn get_pd_integration_key_for_organization(&self, organization_id: u64)
        -> Option<String>;
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

    pub async fn notify_channel(&self, notification: Notification, channel: Channel) -> bool {
        tracing::debug!("notifying channel: {:#?}", channel);
        tracing::debug!("notification: {:#?}", notification);

        let result = match channel {
            Channel::Email => self
                .email_sender
                .notify(notification)
                .await
                .map_err(NotificationDispatcherError::EmailSenderError),
            Channel::Telegram => self
                .telegram
                .notify(notification)
                .await
                .map_err(NotificationDispatcherError::BotError),
            Channel::PagerDuty => self
                .pagerduty
                .notify(notification)
                .await
                .map_err(NotificationDispatcherError::PagerDutyError),
        };

        tracing::debug!("result: {:#?}", result);

        result.is_ok()
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
