use std::{collections::HashMap, sync::Arc};

use sendgrid::{
    v3::{Email, Message, Personalization, Sender},
    SendgridError,
};
use uuid::Uuid;

use crate::{Notification, NotificationConfig, OrganizationDatabase};

#[derive(thiserror::Error, Debug)]
pub enum EmailSenderError {
    #[error(transparent)]
    SendgridError(#[from] SendgridError),
}

pub struct EmailSender {
    pub sender: Sender,
    pub from: String,
    pub db: Arc<Box<dyn OrganizationDatabase>>,
    templates: HashMap<EmailTemplate, String>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum EmailTemplate {
    ActiveSetOut,
    MachineLostContact,
    AVSError,
}

impl EmailTemplate {
    pub fn payload(notification: Notification) -> (Self, HashMap<String, String>) {
        match notification {
            Notification::OutOfActiveSet(address) => (
                Self::ActiveSetOut,
                HashMap::from([("address".to_string(), format!("{address:?}"))]),
            ),
            Notification::MachineLostContact(machine_id) => {
                (Self::MachineLostContact, HashMap::from([("machine_id".to_string(), machine_id)]))
            }
            Notification::AVSError { avs, error } => (
                Self::AVSError,
                HashMap::from([("avs".to_string(), avs), ("error".to_string(), error)]),
            ),
        }
    }
}

impl EmailSender {
    pub fn new(config: &NotificationConfig, db: Arc<Box<dyn OrganizationDatabase>>) -> Self {
        let sender = Sender::new(config.sendgrid_key.to_string(), None);
        let mut templates = HashMap::new();
        templates.insert(
            EmailTemplate::ActiveSetOut,
            config.sendgrid_template_active_set_out.to_string(),
        );
        templates.insert(
            EmailTemplate::MachineLostContact,
            config.sendgrid_template_machine_lost.to_string(),
        );
        templates.insert(EmailTemplate::AVSError, config.sendgrid_template_avs_error.to_string());
        Self { sender, from: config.sendgrid_from.to_string(), db, templates }
    }

    pub async fn notify(
        &self,
        organization: Uuid,
        notification: Notification,
    ) -> Result<(), EmailSenderError> {
        let (template, payload) = EmailTemplate::payload(notification);

        for email in self.db.get_emails_for_organization(organization).await {
            self.sender
                .send(
                    &Message::new(Email::new(&self.from))
                        .set_template_id(&self.templates[&template])
                        .add_personalization(
                            Personalization::new(Email::new(email))
                                .add_dynamic_template_data(payload.clone()),
                        ),
                )
                .await?;
        }
        Ok(())
    }
}
