use std::collections::{HashMap, HashSet};

use ivynet_alerts::Alert;
use sendgrid::{
    v3::{Email, Message, Personalization, Sender},
    SendgridError,
};
use uuid::Uuid;

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

pub struct SendgridParams {
    pub email_template: EmailTemplate,
    pub payload: HashMap<String, String>,
}

pub trait SendgridSend: Clone {
    fn to_sendgrid_template_payload(self) -> SendgridParams;
    fn machine_id(&self) -> Option<Uuid>;
    fn error_type_msg(&self) -> String;
}

impl SendgridSend for Notification {
    /// TODO: Make borrowed version of this to remove the clone downstream.
    fn to_sendgrid_template_payload(self) -> SendgridParams {
        let (email_template, payload) = match self.alert {
            NotificationType::Custom { extra_data, .. } => (
                EmailTemplate::Custom,
                HashMap::from([("message".to_owned(), extra_data.to_string())]),
            ),
            NotificationType::UnregisteredFromActiveSet { node_name, operator, .. } => (
                EmailTemplate::UnregisteredFromActiveSet,
                HashMap::from([
                    ("avs".to_owned(), node_name),
                    ("address".to_owned(), format!("{:?}", operator)),
                ]),
            ),
            NotificationType::MachineNotResponding { .. } => (
                EmailTemplate::MachineNotResponding,
                HashMap::from([(
                    "machine_id".to_owned(),
                    format!("{}", self.machine_id.unwrap_or_default()),
                )]),
            ),
            NotificationType::NodeNotRunning { node_name, .. } => {
                (EmailTemplate::NodeNotRunning, HashMap::from([("avs".to_owned(), node_name)]))
            }
            NotificationType::NoChainInfo { node_name, .. } => {
                (EmailTemplate::NoChainInfo, HashMap::from([("avs".to_owned(), node_name)]))
            }
            NotificationType::NoMetrics { node_name, .. } => {
                (EmailTemplate::NoMetrics, HashMap::from([("avs".to_owned(), node_name)]))
            }
            NotificationType::NoOperatorId { node_name, .. } => {
                (EmailTemplate::NoOperatorId, HashMap::from([("avs".to_owned(), node_name)]))
            }
            NotificationType::HardwareResourceUsage { resource, percent, .. } => (
                EmailTemplate::HardwareResourceUsage,
                HashMap::from([
                    ("machine_id".to_owned(), format!("{}", self.machine_id.unwrap_or_default())),
                    ("resource".to_owned(), resource),
                    ("percent".to_owned(), format!("{percent}")),
                ]),
            ),
            NotificationType::LowPerformanceScore { node_name, performance, .. } => (
                EmailTemplate::LowPerformanceScore,
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
                EmailTemplate::NeedsUpdate,
                HashMap::from([
                    ("avs".to_owned(), node_name),
                    ("current_version".to_owned(), current_version),
                    ("recommended_version".to_owned(), recommended_version),
                ]),
            ),
            NotificationType::ActiveSetNoDeployment { node_name, operator, .. } => (
                EmailTemplate::ActiveSetNoDeployment,
                HashMap::from([
                    ("node_name".to_owned(), node_name),
                    ("address".to_owned(), format!("{:?}", operator)),
                ]),
            ),
            NotificationType::NodeNotResponding { node_name, .. } => (
                EmailTemplate::NodeNotResponding,
                HashMap::from([("node_name".to_owned(), node_name)]),
            ),
            NotificationType::NewEigenAvs {
                name,
                address,
                metadata_uri,
                website,
                twitter,
                description,
                ..
            } => (
                EmailTemplate::NewEigenAvs,
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
                EmailTemplate::UpdatedEigenAvs,
                HashMap::from([
                    ("name".to_owned(), name),
                    ("address".to_owned(), format!("{:?}", address)),
                    ("metadata_uri".to_owned(), metadata_uri),
                    ("website".to_owned(), website),
                    ("twitter".to_owned(), twitter),
                ]),
            ),
        };
        SendgridParams { email_template, payload }
    }

    fn machine_id(&self) -> Option<Uuid> {
        self.machine_id
    }
    fn error_type_msg(&self) -> String {
        match &self.alert {
            Alert::Custom { node_name: _, node_type: _, extra_data } => {
                format!("Custom: {extra_data}")
            }
            Alert::NoMetrics { .. } => "No metrics available".to_string(),
            Alert::NoChainInfo { .. } => "No chain info".to_string(),
            Alert::NoOperatorId { .. } => "No operator id".to_string(),
            Alert::NeedsUpdate {
                node_name: _,
                node_type: _,
                current_version,
                recommended_version,
            } => {
                format!("AVS needs update from {current_version} to {recommended_version}")
            }
            Alert::NodeNotRunning { .. } => "Node not running".to_string(),
            Alert::ActiveSetNoDeployment { node_name: _, node_type: _, operator } => {
                format!("The active set for {operator} is registered, but no metrics is received")
            }
            Alert::UnregisteredFromActiveSet { node_name: _, node_type: _, operator } => {
                format!("Operator {operator:?} unregistered from the active set")
            }
            Alert::MachineNotResponding { machine, .. } => {
                format!("Machine {machine} is not responding")
            }
            Alert::NodeNotResponding { .. } => "AVS is not responding".to_string(),
            Alert::HardwareResourceUsage { resource, percent, .. } => {
                format!("Resource {resource} is used in {percent}%")
            }
            Alert::LowPerformanceScore { node_name: _, node_type: _, performance } => {
                format!("AVS dropped in performace score to {performance}")
            }
            Alert::NewEigenAvs { name, address, metadata_uri, website, twitter, .. } => {
                format!("New EigenLayer AVS: {name} has been detected at {:?} with metadata URI {metadata_uri}. \n Website: {website} \n Twitter: {twitter}", address)
            }
            Alert::UpdatedEigenAvs { name, address, metadata_uri, website, twitter, .. } => {
                format!("Updated EigenLayer AVS: {name} has updated their metadata or address to {:?} with metadata URI {metadata_uri}. \n Website: {website} \n Twitter: {twitter}", address)
            }
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum EmailTemplate {
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
    // Heartbeat variants
    NoClientHeartbeat,
    NoNodeHeartbeat,
    NoMachineHeartbeat,
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

    pub async fn notify(
        &self,
        notification: impl SendgridSend,
        emails: &HashSet<String>,
    ) -> Result<(), EmailSenderError> {
        let SendgridParams { mut email_template, mut payload } =
            notification.clone().to_sendgrid_template_payload();
        if self.templates.len() == 1 {
            email_template = EmailTemplate::Generic;
            payload.insert(
                "machine_id".to_string(),
                format!("{}", notification.machine_id().unwrap_or_default()),
            );
            payload.insert("error_type".to_string(), notification.error_type_msg());
        }
        for email in emails {
            self.sender
                .send(
                    &Message::new(Email::new(&self.from))
                        .set_template_id(&self.templates[&email_template])
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
