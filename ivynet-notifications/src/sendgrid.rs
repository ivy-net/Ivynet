use std::collections::HashMap;

use alerts::Alert;
use sendgrid::{
    v3::{Email, Message, Personalization, Sender},
    SendgridError,
};

use crate::{Notification, NotificationConfig, OrganizationDatabase};

type NotificationType = Alert;

#[derive(thiserror::Error, Debug)]
pub enum EmailSenderError {
    #[error(transparent)]
    SendgridError(#[from] SendgridError),
}

pub struct EmailSender<D: OrganizationDatabase> {
    pub sender: Sender,
    pub from: String,
    pub db: D,
    templates: HashMap<EmailTemplate, String>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum EmailTemplate {
    Custom,
    UnregisteredFromActiveSet,
    MachineNotResponding,
    NodeNotRunning,
    NoChainInfo,
    NoMetrics,
    NoOperatorId,
    HardwareResourceUsage,
    LowPerformaceScore,
    NeedsUpdate,
    ActiveSetNoDeployment,
    NodeNotResponding,
}

impl EmailTemplate {
    pub fn payload(notification: Notification) -> (Self, HashMap<String, String>) {
        match notification.alert {
            NotificationType::Custom(msg) => {
                (Self::Custom, HashMap::from([("message".to_owned(), msg)]))
            }
            NotificationType::UnregisteredFromActiveSet { avs, address } => (
                Self::UnregisteredFromActiveSet,
                HashMap::from([
                    ("avs".to_owned(), avs),
                    ("address".to_owned(), format!("{:?}", address)),
                ]),
            ),
            NotificationType::MachineNotResponding => (
                Self::MachineNotResponding,
                HashMap::from([("machine_id".to_owned(), format!("{}", notification.machine_id))]),
            ),
            NotificationType::NodeNotRunning(avs) => {
                (Self::NodeNotRunning, HashMap::from([("avs".to_owned(), avs)]))
            }
            NotificationType::NoChainInfo(avs) => {
                (Self::NoChainInfo, HashMap::from([("avs".to_owned(), avs)]))
            }
            NotificationType::NoMetrics(avs) => {
                (Self::NoMetrics, HashMap::from([("avs".to_owned(), avs)]))
            }
            NotificationType::NoOperatorId(avs) => {
                (Self::NoOperatorId, HashMap::from([("avs".to_owned(), avs)]))
            }
            NotificationType::HardwareResourceUsage { resource, percent } => (
                Self::HardwareResourceUsage,
                HashMap::from([
                    ("machine_id".to_owned(), format!("{}", notification.machine_id)),
                    ("resource".to_owned(), resource),
                    ("percent".to_owned(), format!("{percent}")),
                ]),
            ),
            NotificationType::LowPerformaceScore { avs, performance } => (
                Self::LowPerformaceScore,
                HashMap::from([
                    ("avs".to_owned(), avs),
                    ("performance".to_owned(), format!("{performance}")),
                ]),
            ),
            NotificationType::NeedsUpdate { avs, current_version, recommended_version } => (
                Self::NeedsUpdate,
                HashMap::from([
                    ("avs".to_owned(), avs),
                    ("current_version".to_owned(), current_version),
                    ("recommended_version".to_owned(), recommended_version),
                ]),
            ),
            NotificationType::ActiveSetNoDeployment { avs, address } => (
                Self::ActiveSetNoDeployment,
                HashMap::from([
                    ("node_name".to_owned(), avs),
                    ("address".to_owned(), format!("{:?}", address)),
                ]),
            ),
            NotificationType::NodeNotResponding(node_name) => {
                (Self::NodeNotResponding, HashMap::from([("node_name".to_owned(), node_name)]))
            }
        }
    }
}

impl<D: OrganizationDatabase> EmailSender<D> {
    pub fn new(config: &NotificationConfig, db: D) -> Self {
        let sender = Sender::new(config.sendgrid_key.to_string(), None);
        let mut templates = HashMap::new();
        templates.insert(EmailTemplate::Custom, config.sendgrid_templates.custom.to_string());
        templates.insert(
            EmailTemplate::UnregisteredFromActiveSet,
            config.sendgrid_templates.unreg_active_set.to_string(),
        );
        templates.insert(
            EmailTemplate::MachineNotResponding,
            config.sendgrid_templates.machine_not_responding.to_string(),
        );
        templates.insert(
            EmailTemplate::NodeNotRunning,
            config.sendgrid_templates.node_not_running.to_string(),
        );
        templates.insert(
            EmailTemplate::NoChainInfo,
            config.sendgrid_templates.no_chain_info.to_string(),
        );
        templates
            .insert(EmailTemplate::NoMetrics, config.sendgrid_templates.no_metrics.to_string());
        templates
            .insert(EmailTemplate::NoOperatorId, config.sendgrid_templates.no_operator.to_string());
        templates.insert(
            EmailTemplate::HardwareResourceUsage,
            config.sendgrid_templates.hw_res_usage.to_string(),
        );
        templates.insert(
            EmailTemplate::LowPerformaceScore,
            config.sendgrid_templates.low_perf.to_string(),
        );
        templates
            .insert(EmailTemplate::NeedsUpdate, config.sendgrid_templates.needs_update.to_string());
        Self { sender, from: config.sendgrid_from.to_string(), db, templates }
    }

    pub async fn notify(&self, notification: Notification) -> Result<(), EmailSenderError> {
        let organization = notification.organization;
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
