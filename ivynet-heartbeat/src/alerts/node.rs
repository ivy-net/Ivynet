use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use ivynet_database::error::DatabaseError;
use ivynet_notifications::{
    pagerduty::{Action, Event, PagerDutySend, Payload, Severity},
    sendgrid::{EmailTemplate, SendgridParams, SendgridSend},
    telegram::TelegramSend,
    NotificationSend,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::NodeId;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeHeartbeatAlert {
    pub node_id: NodeId,
    pub last_response_time: DateTime<Utc>,
}

pub struct DbNodeHeartbeatAlert {
    pub node_id: String,
    pub last_response_time: NaiveDateTime,
}

impl TryFrom<DbNodeHeartbeatAlert> for NodeHeartbeatAlert {
    type Error = DatabaseError;

    fn try_from(value: DbNodeHeartbeatAlert) -> Result<Self, Self::Error> {
        let node_id_raw = value.node_id.clone();
        let parts = value.node_id.split_once(':');
        if let Some((machine, name)) = parts {
            Ok(Self {
                node_id: NodeId {
                    machine: Uuid::parse_str(machine)
                        .map_err(|_| DatabaseError::NodeIdParseError(node_id_raw))?,
                    name: name.to_string(),
                },
                last_response_time: DateTime::<Utc>::from_naive_utc_and_offset(
                    value.last_response_time,
                    Utc,
                ),
            })
        } else {
            Err(DatabaseError::NodeIdParseError(node_id_raw))
        }
    }
}

impl NodeHeartbeatAlert {
    pub async fn get(pool: &PgPool, node_id: NodeId) -> Result<Option<Self>, DatabaseError> {
        let alert = sqlx::query_as!(
            DbNodeHeartbeatAlert,
            "SELECT node_id, last_response_time FROM node_heartbeat WHERE node_id = $1",
            format!("{}:{}", node_id.machine, node_id.name)
        )
        .fetch_optional(pool)
        .await?;
        match alert {
            Some(alert) => Ok(Some(alert.try_into()?)),
            None => Ok(None),
        }
    }

    pub async fn insert(
        pool: &PgPool,
        alert: Self,
        organization_id: i64,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "INSERT INTO node_heartbeat (node_id, organization_id, last_response_time) VALUES ($1, $2, $3)",
            format!("{}:{}", alert.node_id.machine, alert.node_id.name),
            organization_id,
            alert.last_response_time.naive_utc()
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &PgPool, node_id: NodeId) -> Result<(), DatabaseError> {
        sqlx::query!("DELETE FROM node_heartbeat WHERE node_id = $1", node_id.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }
}

impl NotificationSend for NodeHeartbeatAlert {}

impl PagerDutySend for NodeHeartbeatAlert {
    fn to_pagerduty_message(&self) -> String {
        let last_response_time = self.last_response_time.to_utc().to_string();
        format!(
            "Failed to receive heartbeat from node. ID: {}:{} | Last heartbeat sent at {last_response_time} UTC",
            self.node_id.machine, self.node_id.name
        )
    }
}

impl From<NodeHeartbeatAlert> for Event {
    fn from(value: NodeHeartbeatAlert) -> Self {
        Self {
            routing_key: "".to_owned(),
            event_action: Action::Trigger,
            dedup_key: Uuid::new_v5(&Uuid::NAMESPACE_OID, value.node_id.to_string().as_bytes()),
            client: Some(value.node_id.name.clone()),
            payload: Payload {
                severity: Severity::Error,
                source: "IvyNet".to_owned(),
                summary: value.to_pagerduty_message(),
                timestamp: chrono::Utc::now(),
                component: Some(format!("Node:{}", value.node_id.name)),
            },
        }
    }
}

impl SendgridSend for NodeHeartbeatAlert {
    fn to_sendgrid_template_payload(self) -> SendgridParams {
        SendgridParams {
            email_template: EmailTemplate::NoNodeHeartbeat,
            payload: HashMap::from([
                ("node_name".to_string(), self.node_id.name.clone()),
                ("machine_id".to_string(), self.node_id.machine.to_string()),
                ("last_response_time".to_string(), self.last_response_time.to_string()),
            ]),
        }
    }

    fn machine_id(&self) -> Option<Uuid> {
        Some(self.node_id.machine)
    }

    fn error_type_msg(&self) -> String {
        format!(
            "Failed to receive heartbeat from node. Name: {}, Machine: {}",
            self.node_id.name, self.node_id.machine
        )
    }
}

impl TelegramSend for NodeHeartbeatAlert {
    fn to_telegram_message(&self) -> String {
        let last_response_time = self.last_response_time.to_utc().to_string();
        format!(
            "❗ *Node Heartbeat Alert* ❗️\nFailed to receive heartbeat from node\nName: `{}`\nMachine: `{}`\nLast heartbeat: `{}`",
            Self::escape_markdown_v2(&self.node_id.name),
            Self::escape_markdown_v2(&self.node_id.machine.to_string()),
            Self::escape_markdown_v2(&last_response_time)
        )
    }
}
