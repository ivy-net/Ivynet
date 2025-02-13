use ethers::types::H160;
use sendgrid::EmailSender;
use telegram::TelegramBot;
use uuid::Uuid;

pub mod sendgrid;
pub mod telegram;

#[derive(thiserror::Error, Debug)]
pub enum NotificationDispatcherError {
    #[error(transparent)]
    BotError(#[from] telegram::BotError),

    #[error(transparent)]
    EmailSenderError(#[from] sendgrid::EmailSenderError),
}

#[derive(Debug, Clone)]
pub enum Notification {
    OutOfActiveSet(H160),
    MachineLostContact(String),
    AVSError { avs: String, error: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Channel {
    Telegram,
    Email,
}

#[derive(Clone, Debug)]
pub struct NotificationConfig<'a> {
    pub telegram_token: &'a str,
    pub sendgrid_key: &'a str,
    pub sendgrid_from: &'a str,
    pub sendgrid_template_active_set_out: &'a str,
    pub sendgrid_template_machine_lost: &'a str,
    pub sendgrid_template_avs_error: &'a str,
}

pub struct NotificationDispatcher<D: OrganizationDatabase> {
    pub telegram: TelegramBot<D>,
    pub email_sender: EmailSender<D>,
}

#[async_trait::async_trait]
pub trait OrganizationDatabase: Send + Sync + Clone + 'static {
    async fn register_chat(&self, chat_id: &str, email: &str, password: &str) -> bool;
    async fn unregister_chat(&self, chat_id: &str) -> bool;
    async fn get_emails_for_organization(&self, organization_id: Uuid) -> Vec<String>;
    async fn get_chats_for_organization(&self, organization_id: Uuid) -> Vec<String>;
}

impl<D: OrganizationDatabase> NotificationDispatcher<D> {
    pub fn new(config: NotificationConfig, db: D) -> Self {
        Self {
            telegram: TelegramBot::<D>::new(config.telegram_token, db.clone()),
            email_sender: EmailSender::new(&config, db),
        }
    }

    pub async fn serve(&self) -> Result<(), NotificationDispatcherError> {
        self.telegram.serve().await?;
        Ok(())
    }

    pub async fn notify(
        &self,
        organization: Uuid,
        notification: Notification,
        channels: Vec<Channel>,
    ) -> Result<(), NotificationDispatcherError> {
        if channels.contains(&Channel::Telegram) {
            self.telegram.notify(organization, notification.clone()).await?;
        }
        if channels.contains(&Channel::Email) {
            self.email_sender.notify(organization, notification).await?;
        }
        Ok(())
    }
}
