use std::sync::Arc;

use teloxide::{prelude::*, utils::command::BotCommands};

use tracing::warn;
use uuid::Uuid;

use crate::OrganizationDatabase;

use super::Notification;

#[derive(thiserror::Error, Debug)]
pub enum BotError {
    #[error(transparent)]
    RequestError(#[from] teloxide::RequestError),
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
        description = "Register this chat to notifications"
    )]
    Register { email: String, password: String },

    #[command(
        rename_rule = "lowercase",
        parse_with = "split",
        description = "Unregister previously registered chat for notifications"
    )]
    Unregister,
}

pub struct TelegramBot {
    pub bot: Bot,
    pub db: Arc<Box<dyn OrganizationDatabase>>,
}

impl TelegramBot {
    pub fn new(bot_key: &str, db: Arc<Box<dyn OrganizationDatabase>>) -> Self {
        Self { bot: Bot::new(bot_key), db }
    }

    pub async fn serve(&self) -> Result<(), BotError> {
        let handler = Update::filter_message()
            .branch(dptree::entry().filter_command::<BotCommand>().endpoint(command_handler));
        Dispatcher::builder(self.bot.clone(), handler)
            .dependencies(dptree::deps![self.db.clone()])
            .default_handler(|upd| async move {
                warn!("Unhandled update: {:?}", upd);
            })
            // If the dispatcher fails for some reason, execute this handler.
            .error_handler(LoggingErrorHandler::with_custom_text(
                "An error has occurred in the dispatcher",
            ))
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;
        Ok(())
    }

    pub async fn notify(
        &self,
        organization: Uuid,
        notification: Notification,
    ) -> Result<(), BotError> {
        let message = match notification {
            Notification::OutOfActiveSet(address) => {
                format!("Address {address:?} has been removed from the active set")
            }
            Notification::MachineLostContact(machine_id) => {
                format!("Machine '{machine_id}' has lost connection with our backend")
            }
            Notification::AVSError { avs, error } => format!("AVS ERROR: [{avs}]: {error}"),
        };

        for chat in self.db.get_chats_for_organization(organization).await {
            self.bot.send_message(chat, &message).await?;
        }
        Ok(())
    }
}

async fn command_handler(
    db: Arc<Box<dyn OrganizationDatabase>>,
    bot: Bot,
    msg: Message,
    cmd: BotCommand,
) -> ResponseResult<()> {
    match cmd {
        BotCommand::Help => {
            bot.send_message(msg.chat.id, BotCommand::descriptions().to_string()).await?;
        }
        BotCommand::Register { email, password } => {
            if db.register_chat(msg.chat.id.to_string().as_str(), &email, &password).await {
                bot.send_message(msg.chat.id, "Registration successful.").await?;
            } else {
                bot.send_message(msg.chat.id, "Registration failed.").await?;
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
