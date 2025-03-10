use std::time::Duration;

use ivynet_alerts::Alert;
use teloxide::{
    dispatching::UpdateHandler, prelude::*, types::ParseMode, utils::command::BotCommands,
};
use tokio::time::sleep;
use tracing::warn;
use uuid::Uuid;

use crate::OrganizationDatabase;

use super::Notification;

type NotificationType = Alert;

#[derive(thiserror::Error, Debug)]
pub enum BotError {
    #[error(transparent)]
    RequestError(#[from] teloxide::RequestError),

    #[error("No bot configured")]
    NoBotConfigured,
}

/// These commands are supported:
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum BotCommand {
    #[command(rename_rule = "lowercase", parse_with = "split", description = "Display this text")]
    Help,
    #[command(
        rename_rule = "lowercase",
        parse_with = "split",
        description = "Register this chat to notifications using /register <email> <password>. Your message will be deleted after registration."
    )]
    Register { email: String, password: String },

    #[command(
        rename_rule = "lowercase",
        parse_with = "split",
        description = "Unregister previously registered chat for notifications"
    )]
    Unregister,
}

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub struct TelegramBot<D: OrganizationDatabase> {
    pub bot: Option<Bot>,
    pub db: D,
}

pub trait TelegramBotApi {
    fn new(bot_key: &str, db: impl OrganizationDatabase) -> Self;
    fn serve(&self) -> Result<(), BotError>;
    fn notify(&self, organization: Uuid, notification: Notification) -> Result<(), BotError>;
}

impl<D: OrganizationDatabase> TelegramBot<D> {
    pub fn new(bot_key: &str, db: D) -> Self {
        Self { bot: if bot_key.is_empty() { None } else { Some(Bot::new(bot_key)) }, db }
    }

    pub async fn serve(&self) -> Result<(), BotError> {
        if let Some(bot) = &self.bot {
            let handler = Self::handler_tree();
            Dispatcher::builder(bot.clone(), handler)
                .dependencies(dptree::deps![self.db.clone()])
                .default_handler(|upd| async move {
                    warn!("Unhandled update: {:#?}", upd);
                })
                // If the dispatcher fails for some reason, execute this handler.
                .error_handler(LoggingErrorHandler::with_custom_text(
                    "An error has occurred in the dispatcher",
                ))
                .enable_ctrlc_handler()
                .build()
                .dispatch()
                .await;
        } else {
            loop {
                sleep(Duration::from_secs(100)).await;
            }
        }
        Ok(())
    }

    fn escape_markdown_v2(text: &str) -> String {
        let special_chars = [
            '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.',
            '!',
        ];
        let mut escaped = String::with_capacity(text.len() * 2);
        for c in text.chars() {
            if special_chars.contains(&c) {
                escaped.push('\\');
            }
            escaped.push(c);
        }
        escaped
    }

    pub async fn notify(&self, notification: Notification) -> Result<(), BotError> {
        let message = match notification.alert {
            NotificationType::UnregisteredFromActiveSet { node_name, node_type: _, operator } => {
                format!(
                    "❗ *Operator Unregistered from Active Set* ❗️\nAddress `{}` has been removed from the active set for node `{}`\n🔗 [Machine Details](http://ivynet\\.dev/machines/{})",
                    Self::escape_markdown_v2(&format!("{:?}", operator)),
                    Self::escape_markdown_v2(&node_name),
                    notification.machine_id.unwrap_or_default()
                )
            }
            NotificationType::MachineNotResponding => {
                format!(
                    "❗ *Machine Not Responding* ❗️\nMachine `{}` has lost connection with our backend\n🔗 [Machine Details](http://ivynet\\.dev/machines/{})",
                    Self::escape_markdown_v2(&format!("{:?}", notification.machine_id.unwrap_or_default())),
                    notification.machine_id.unwrap_or_default()
                )
            }
            NotificationType::Custom { node_name, node_type: _, extra_data } => {
                format!(
                    "❗ *Custom Alert* ❗️\nNode `{}` has triggered a custom alert with custom data: `{}`\n🔗 [Machine Details](http://ivynet\\.dev/machines/{})",
                    Self::escape_markdown_v2(&node_name),
                    Self::escape_markdown_v2(&extra_data.to_string()),
                    notification.machine_id.unwrap_or_default()
                )
            }
            NotificationType::NodeNotRunning { node_name, node_type: _ } => {
                format!(
                    "❗ *Node Not Running* ❗️\nNode `{}` is not running on machine `{}`\n🔗 [Machine Details](http://ivynet\\.dev/machines/{})",
                    Self::escape_markdown_v2(&node_name),
                    Self::escape_markdown_v2(&format!("{:?}", notification.machine_id.unwrap_or_default())),
                    notification.machine_id.unwrap_or_default()
                )
            }
            NotificationType::NoChainInfo { node_name, node_type: _ } => {
                format!(
                    "❗ *No Chain Info* ❗️ \nNode `{}` has no chain information \n🔗 [Machine Details](http://ivynet\\.dev/machines/{})",
                    Self::escape_markdown_v2(&node_name),
                    notification.machine_id.unwrap_or_default()
                )
            }
            NotificationType::NoMetrics { node_name, node_type: _ } => {
                format!(
                    "❗ *No Metrics* ❗️\nNode `{}` is not reporting any metrics\n🔗 [Machine Details](http://ivynet\\.dev/machines/{})",
                    Self::escape_markdown_v2(&node_name),
                    notification.machine_id.unwrap_or_default()
                )
            }
            NotificationType::NoOperatorId { node_name, node_type: _ } => {
                format!(
                    "❗ *No Operator ID* ❗️\nNode `{}` has no associated operator ID\n🔗 [Machine Details](http://ivynet\\.dev/machines/{})",
                    Self::escape_markdown_v2(&node_name),
                    notification.machine_id.unwrap_or_default()
                )
            }
            NotificationType::HardwareResourceUsage { machine, resource, percent } => {
                format!(
                    "❗ *Hardware Resource Usage* ❗️\nMachine `{}` has used over `{}%` of `{}`\n🔗 [Machine Details](http://ivynet\\.dev/machines/{})",
                    Self::escape_markdown_v2(&format!("{:?}", machine)),
                    percent,
                    Self::escape_markdown_v2(&resource),
                    machine
                )
            }
            NotificationType::LowPerformanceScore { node_name, node_type: _, performance } => {
                format!(
                    "❗ *Low Performance Score* ❗️\nNode `{}` has a LOW performance score of `{}`\n🔗 [Machine Details](http://ivynet\\.dev/machines/{})",
                    Self::escape_markdown_v2(&node_name),
                    performance,
                    notification.machine_id.unwrap_or_default()
                )
            }
            NotificationType::NeedsUpdate {
                node_name,
                node_type: _,
                current_version,
                recommended_version,
            } => {
                format!(
                    "❗ *Node Update Available* ❗️\nNode `{}` is running version `{}` but version `{}` is available\n🔗 [Machine Details](http://ivynet\\.dev/machines/{})",
                    Self::escape_markdown_v2(&node_name),
                    Self::escape_markdown_v2(&current_version),
                    Self::escape_markdown_v2(&recommended_version),
                    notification.machine_id.unwrap_or_default()
                )
            }
            NotificationType::ActiveSetNoDeployment { node_name, .. } => {
                format!(
                    "❗ *Active Set No Deployment* ❗️\nNode `{}` is in the active set, but the node is either not deployed or not responding\n🔗 [Machine Details](http://ivynet\\.dev/machines/{})",
                    Self::escape_markdown_v2(&node_name),
                    notification.machine_id.unwrap_or_default()
                )
            }
            NotificationType::NodeNotResponding { node_name, .. } => {
                format!(
                    "❗ *Node Not Responding* ❗️\nNode `{}` is not responding\n🔗 [Machine Details](http://ivynet\\.dev/machines/{})",
                    Self::escape_markdown_v2(&node_name),
                    notification.machine_id.unwrap_or_default()
                )
            }
            NotificationType::NewEigenAvs {
                name,
                address,
                metadata_uri,
                website,
                twitter,
                description,
                ..
            } => {
                format!(
                    "❗ *New EigenLayer AVS* ❗️\n\nNew EigenLayer AVS: {} has been detected at {} with metadata URI {}\n\nWebsite: {}\nTwitter: {}\nDescription: {}",
                    Self::escape_markdown_v2(&name),
                    Self::escape_markdown_v2(&format!("{:?}", address)),
                    Self::escape_markdown_v2(&metadata_uri),
                    Self::escape_markdown_v2(&website),
                    Self::escape_markdown_v2(&twitter),
                    Self::escape_markdown_v2(&description)
                )
            }
            NotificationType::UpdatedEigenAvs {
                name,
                address,
                metadata_uri,
                website,
                twitter,
                ..
            } => {
                format!(
                    "❗ *Updated EigenLayer AVS* ❗️\n\nUpdated EigenLayer AVS: {} has updated their metadata or address to {} with metadata URI {}\n\nWebsite: {}\nTwitter: {}",
                    Self::escape_markdown_v2(&name),
                    Self::escape_markdown_v2(&format!("{:?}", address)),
                    Self::escape_markdown_v2(&metadata_uri),
                    Self::escape_markdown_v2(&website),
                    Self::escape_markdown_v2(&twitter)
                )
            }
        };

        if let Some(bot) = &self.bot {
            for chat in self.db.get_chats_for_organization(notification.organization).await {
                bot.parse_mode(ParseMode::MarkdownV2).send_message(chat, &message).await?;
            }
            Ok(())
        } else {
            Err(BotError::NoBotConfigured)
        }
    }

    fn handler_tree() -> UpdateHandler<teloxide::RequestError> {
        Update::filter_message()
            .branch(dptree::entry().filter_command::<BotCommand>().endpoint(command_handler::<D>))
    }
}

async fn command_handler<D: OrganizationDatabase>(
    db: D,
    bot: Bot,
    msg: Message,
    cmd: BotCommand,
) -> ResponseResult<()> {
    match cmd {
        BotCommand::Help => {
            bot.send_message(msg.chat.id, BotCommand::descriptions().to_string()).await?;
        }
        BotCommand::Register { email, password } => {
            bot.delete_message(msg.chat.id, msg.id).await?;
            if db.register_chat(msg.chat.id.to_string().as_str(), &email, &password).await {
                bot.send_message(msg.chat.id, "Registration successful.").await?;
            } else {
                bot.send_message(
                    msg.chat.id,
                    "Registration failed. Please check that your email and password are correct.",
                )
                .await?;
            }
        }
        BotCommand::Unregister => {
            if db.unregister_chat(msg.chat.id.to_string().as_str()).await {
                bot.send_message(msg.chat.id, "You have successfully unregistered this chat.")
                    .await?;
            } else {
                bot.send_message(msg.chat.id, "This chat was not registered.").await?;
            }
        }
    };

    Ok(())
}

impl<D: OrganizationDatabase> TelegramBot<D> {
    pub fn wrapped_handler_tree(
    ) -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
        Update::filter_message().branch(
            dptree::entry().filter_command::<BotCommand>().endpoint(wrapped_command_handler::<D>),
        )
    }
}

async fn wrapped_command_handler<D: OrganizationDatabase>(
    db: D,
    bot: Bot,
    message: Message,
    cmd: BotCommand,
) -> HandlerResult {
    command_handler(db, bot, message, cmd).await.map_err(Into::into)
}

#[cfg(test)]
mod telegram_bot_test {

    use std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    };

    use tokio::sync::Mutex;

    use super::*;

    use teloxide_tests::{MockBot, MockMessageText};

    static MOCK_ORGANIZATION_ID: u64 = 1;

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
            None
        }
    }

    #[tokio::test]
    async fn test_command_handler() {
        let mock_message = MockMessageText::new().text("/help");

        let db = MockDb::new();

        let bot = MockBot::new(mock_message, TelegramBot::<MockDb>::wrapped_handler_tree());
        bot.dependencies(dptree::deps![db]);
        bot.dispatch().await;

        let responses = bot.get_responses();
        let message = responses
            .sent_messages // This is a list of all sent messages. Be warned, editing or deleting
            // messages do not affect this list!
            .last()
            .expect("No sent messages were detected!");

        assert_eq!(message.text(), Some(BotCommand::descriptions().to_string().as_str()));
    }

    #[tokio::test]
    async fn test_registration_commands() {
        let mock_message = MockMessageText::new().text("/register test@email.com s0mePass");

        let db = MockDb::new();

        let bot = MockBot::new(mock_message, TelegramBot::<MockDb>::wrapped_handler_tree());
        bot.dependencies(dptree::deps![db.clone()]);
        bot.dispatch().await;

        let responses = bot.get_responses();
        let message = responses.sent_messages.last().expect("No sent messages were detected!");

        assert_eq!(message.text(), Some("Registration successful."));
        assert_eq!(db.get_chats_for_organization(MOCK_ORGANIZATION_ID).await.len(), 1);

        let mock_unregister_message =
            MockMessageText::new().chat(message.chat.clone()).text("/unregister");
        bot.update(mock_unregister_message);
        bot.dispatch().await;

        let responses = bot.get_responses();
        let message = responses.sent_messages.last().expect("No sent messages were detected!");

        assert_eq!(message.text(), Some("You have successfully unregistered this chat."));
        assert_eq!(db.get_chats_for_organization(MOCK_ORGANIZATION_ID).await.len(), 0);
    }

    #[tokio::test]
    async fn test_bad_unregistration_command() {
        let db = MockDb::new();

        let mock_message = MockMessageText::new().text("/unregister");
        let bot = MockBot::new(mock_message, TelegramBot::<MockDb>::wrapped_handler_tree());
        bot.dependencies(dptree::deps![db.clone()]);
        bot.dispatch().await;

        let responses = bot.get_responses();
        let message = responses.sent_messages.last().expect("No sent messages were detected!");

        assert_eq!(message.text(), Some("This chat was not registered."));
        assert_eq!(db.get_chats_for_organization(MOCK_ORGANIZATION_ID).await.len(), 0);
    }

    #[tokio::test]
    async fn test_event_propagation() {
        let mock_message = MockMessageText::new().text("/register test@email.com s0mePass");

        let db = MockDb::new();

        let bot = MockBot::new(mock_message, TelegramBot::<MockDb>::wrapped_handler_tree());
        bot.dependencies(dptree::deps![db.clone()]);
        bot.dispatch().await;

        let responses = bot.get_responses();
        let message = responses.sent_messages.last().expect("No sent messages were detected!");

        assert_eq!(message.text(), Some("Registration successful."));
        assert_eq!(db.get_chats_for_organization(MOCK_ORGANIZATION_ID).await.len(), 1);
    }
}
