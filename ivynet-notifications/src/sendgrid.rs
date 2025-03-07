use std::collections::HashMap;

use ivynet_alerts::Alert;
use sendgrid::{
    v3::{Email, Message, Personalization, Sender},
    SendgridError,
};

use crate::{Notification, NotificationConfig, OrganizationDatabase, SendgridTemplates};

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
    Generic,
    UnregisteredFromActiveSet,
    MachineNotResponding,
    NodeNotRunning,
    NoChainInfo,
    NoMetrics,
    NoOperatorId,
    HardwareResourceUsage,
    LowPerformanceScore,
    NeedsUpdate,
    ActiveSetNoDeployment,
    NodeNotResponding,
    NewEigenAvs,
    UpdatedEigenAvs,
}

impl EmailTemplate {
    pub fn payload(notification: Notification) -> (Self, HashMap<String, String>) {
        match notification.alert {
            NotificationType::Custom { extra_data, .. } => {
                (Self::Custom, HashMap::from([("message".to_owned(), extra_data.to_string())]))
            }
            NotificationType::UnregisteredFromActiveSet { node_name, operator, .. } => (
                Self::UnregisteredFromActiveSet,
                HashMap::from([
                    ("avs".to_owned(), node_name),
                    ("address".to_owned(), format!("{:?}", operator)),
                ]),
            ),
            NotificationType::MachineNotResponding => (
                Self::MachineNotResponding,
                HashMap::from([(
                    "machine_id".to_owned(),
                    format!("{}", notification.machine_id.unwrap_or_default()),
                )]),
            ),
            NotificationType::NodeNotRunning { node_name, .. } => {
                (Self::NodeNotRunning, HashMap::from([("avs".to_owned(), node_name)]))
            }
            NotificationType::NoChainInfo { node_name, .. } => {
                (Self::NoChainInfo, HashMap::from([("avs".to_owned(), node_name)]))
            }
            NotificationType::NoMetrics { node_name, .. } => {
                (Self::NoMetrics, HashMap::from([("avs".to_owned(), node_name)]))
            }
            NotificationType::NoOperatorId { node_name, .. } => {
                (Self::NoOperatorId, HashMap::from([("avs".to_owned(), node_name)]))
            }
            NotificationType::HardwareResourceUsage { resource, percent, .. } => (
                Self::HardwareResourceUsage,
                HashMap::from([
                    (
                        "machine_id".to_owned(),
                        format!("{}", notification.machine_id.unwrap_or_default()),
                    ),
                    ("resource".to_owned(), resource),
                    ("percent".to_owned(), format!("{percent}")),
                ]),
            ),
            NotificationType::LowPerformanceScore { node_name, performance, .. } => (
                Self::LowPerformanceScore,
                HashMap::from([
                    ("avs".to_owned(), node_name),
                    ("performance".to_owned(), format!("{performance}")),
                ]),
            ),
            NotificationType::NeedsUpdate {
                node_name,
                current_version,
                recommended_version,
                ..
            } => (
                Self::NeedsUpdate,
                HashMap::from([
                    ("avs".to_owned(), node_name),
                    ("current_version".to_owned(), current_version),
                    ("recommended_version".to_owned(), recommended_version),
                ]),
            ),
            NotificationType::ActiveSetNoDeployment { node_name, operator, .. } => (
                Self::ActiveSetNoDeployment,
                HashMap::from([
                    ("node_name".to_owned(), node_name),
                    ("address".to_owned(), format!("{:?}", operator)),
                ]),
            ),
            NotificationType::NodeNotResponding { node_name, .. } => {
                (Self::NodeNotResponding, HashMap::from([("node_name".to_owned(), node_name)]))
            }
            NotificationType::NewEigenAvs {
                name,
                address,
                metadata_uri,
                website,
                twitter,
                description,
                ..
            } => (
                Self::NewEigenAvs,
                HashMap::from([
                    ("name".to_owned(), name),
                    ("address".to_owned(), format!("{:?}", address)),
                    ("metadata_uri".to_owned(), metadata_uri),
                    ("website".to_owned(), website),
                    ("twitter".to_owned(), twitter),
                    ("description".to_owned(), description),
                ]),
            ),
            NotificationType::UpdatedEigenAvs {
                name,
                address,
                metadata_uri,
                website,
                twitter,
                ..
            } => (
                Self::UpdatedEigenAvs,
                HashMap::from([
                    ("name".to_owned(), name),
                    ("address".to_owned(), format!("{:?}", address)),
                    ("metadata_uri".to_owned(), metadata_uri),
                    ("website".to_owned(), website),
                    ("twitter".to_owned(), twitter),
                ]),
            ),
        }
    }
}

impl<D: OrganizationDatabase> EmailSender<D> {
    pub fn new(config: &NotificationConfig, db: D) -> Self {
        let sender = Sender::new(config.sendgrid_key.to_string(), None);
        let mut templates = HashMap::new();
        match &config.sendgrid_templates {
            SendgridTemplates::Generic(generic_template) => {
                templates.insert(EmailTemplate::Generic, generic_template.clone());
            }
            SendgridTemplates::Specific(sendgrid_templates) => {
                templates.insert(EmailTemplate::Custom, sendgrid_templates.custom.to_string());
                templates.insert(
                    EmailTemplate::UnregisteredFromActiveSet,
                    sendgrid_templates.unreg_active_set.to_string(),
                );
                templates.insert(
                    EmailTemplate::MachineNotResponding,
                    sendgrid_templates.machine_not_responding.to_string(),
                );
                templates.insert(
                    EmailTemplate::NodeNotRunning,
                    sendgrid_templates.node_not_running.to_string(),
                );
                templates.insert(
                    EmailTemplate::NoChainInfo,
                    sendgrid_templates.no_chain_info.to_string(),
                );
                templates
                    .insert(EmailTemplate::NoMetrics, sendgrid_templates.no_metrics.to_string());
                templates.insert(
                    EmailTemplate::NoOperatorId,
                    sendgrid_templates.no_operator.to_string(),
                );
                templates.insert(
                    EmailTemplate::HardwareResourceUsage,
                    sendgrid_templates.hw_res_usage.to_string(),
                );
                templates.insert(
                    EmailTemplate::LowPerformanceScore,
                    sendgrid_templates.low_perf.to_string(),
                );
                templates.insert(
                    EmailTemplate::NeedsUpdate,
                    sendgrid_templates.needs_update.to_string(),
                );
            }
        }
        Self { sender, from: config.sendgrid_from.to_string(), db, templates }
    }

    pub async fn notify(&self, notification: Notification) -> Result<(), EmailSenderError> {
        let organization = notification.organization;
        // If there is only generic template

        let (mut template, mut payload) = EmailTemplate::payload(notification.clone());
        if self.templates.len() == 1 {
            template = EmailTemplate::Generic;
            payload.insert(
                "machine_id".to_string(),
                format!("{}", notification.machine_id.unwrap_or_default()),
            );
            payload.insert(
                "error_type".to_string(),
                match notification.alert {
                    Alert::Custom{node_name: _, node_type: _, extra_data} => format!("Custom: {extra_data}"),
                    Alert::NoMetrics {..} => "No metrics available".to_string(),
                    Alert::NoChainInfo {..} => "No chain info".to_string(),
                    Alert::NoOperatorId {..} => "No operator id".to_string(),
                    Alert::NeedsUpdate { node_name: _, node_type: _, current_version, recommended_version } => {
                        format!("AVS needs update from {current_version} to {recommended_version}")
                    }
                    Alert::NodeNotRunning {..} => "Node not running".to_string(),
                    Alert::ActiveSetNoDeployment { node_name: _, node_type: _, operator } => {
                        format!("The active set for {operator} is registered, but no metrics is received")
                    }
                    Alert::UnregisteredFromActiveSet { node_name: _, node_type: _, operator } => {
                        format!("Operator {operator:?} unregistered from the active set")
                    }
                    Alert::MachineNotResponding => "Machine is not responding".to_string(),
                    Alert::NodeNotResponding {..} => "AVS is not responding".to_string(),
                    Alert::HardwareResourceUsage { resource, percent, .. } => {
                        format!("Resource {resource} is used in {percent}%")
                    }
                    Alert::LowPerformanceScore { node_name: _, node_type: _, performance } => {
                        format!("AVS dropped in performace score to {performance}")
                    }
                    Alert::NewEigenAvs { name, address, metadata_uri, website, twitter, .. } => {
                        format!("New EigenLayer AVS: {name} has been detected at {address} with metadata URI {metadata_uri}. \n Website: {website} \n Twitter: {twitter}")
                    }
                    Alert::UpdatedEigenAvs { name, address, metadata_uri, website, twitter, .. } => {
                        format!("Updated EigenLayer AVS: {name} has updated their metadata or address to {address} with metadata URI {metadata_uri}. \n Website: {website} \n Twitter: {twitter}")
                    }
                },
            );
        }
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
