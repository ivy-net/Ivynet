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

use crate::MachineId;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MachineHeartbeatAlert {
    pub machine_id: MachineId,
    pub last_response_time: DateTime<Utc>,
}

pub struct DbMachineHeartbeatAlert {
    pub machine_id: Uuid,
    pub last_response_time: NaiveDateTime,
}

impl From<DbMachineHeartbeatAlert> for MachineHeartbeatAlert {
    fn from(value: DbMachineHeartbeatAlert) -> Self {
        Self {
            machine_id: MachineId(value.machine_id),
            last_response_time: DateTime::<Utc>::from_naive_utc_and_offset(
                value.last_response_time,
                Utc,
            ),
        }
    }
}

impl MachineHeartbeatAlert {
    pub async fn get(pool: &PgPool, machine_id: MachineId) -> Result<Option<Self>, DatabaseError> {
        let alert = sqlx::query_as!(
            DbMachineHeartbeatAlert,
            "SELECT machine_id, last_response_time FROM machine_heartbeat WHERE machine_id = $1",
            machine_id.0
        )
        .fetch_optional(pool)
        .await?;

        Ok(alert.map(|a| a.into()))
    }

    pub async fn insert(
        pool: &PgPool,
        alert: Self,
        organization_id: i64,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "INSERT INTO machine_heartbeat (machine_id, organization_id, last_response_time) VALUES ($1, $2, $3)",
            alert.machine_id.0,
            organization_id,
            alert.last_response_time.naive_utc()
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &PgPool, machine_id: MachineId) -> Result<(), DatabaseError> {
        sqlx::query!("DELETE FROM machine_heartbeat WHERE machine_id = $1", machine_id.0)
            .execute(pool)
            .await?;

        Ok(())
    }
}

impl NotificationSend for MachineHeartbeatAlert {}

impl PagerDutySend for MachineHeartbeatAlert {
    fn to_pagerduty_message(&self) -> String {
        let last_response_time = self.last_response_time.to_utc().to_string();
        format!("Failed to receive heartbeat from machine. ID: {} | Last heartbeat sent at {last_response_time} UTC", self.machine_id.0)
    }
}

impl From<MachineHeartbeatAlert> for Event {
    fn from(value: MachineHeartbeatAlert) -> Self {
        Self {
            routing_key: "".to_owned(),
            event_action: Action::Trigger,
            dedup_key: Uuid::new_v5(&Uuid::NAMESPACE_OID, value.machine_id.0.as_bytes()),
            client: Some(value.machine_id.0.to_string()),
            payload: Payload {
                severity: Severity::Error,
                source: "IvyNet".to_owned(),
                summary: value.to_pagerduty_message(),
                timestamp: chrono::Utc::now(),
                component: Some("Machine".to_owned()),
            },
        }
    }
}

impl SendgridSend for MachineHeartbeatAlert {
    fn to_sendgrid_template_payload(self) -> SendgridParams {
        SendgridParams {
            email_template: EmailTemplate::NoMachineHeartbeat,
            payload: HashMap::from([
                ("machine_id".to_string(), self.machine_id.0.to_string()),
                ("last_response_time".to_string(), self.last_response_time.to_string()),
            ]),
        }
    }

    fn machine_id(&self) -> Option<Uuid> {
        Some(self.machine_id.0)
    }

    fn error_type_msg(&self) -> String {
        format!("Failed to receive heartbeat from machine. ID: {}", self.machine_id.0)
    }
}

impl TelegramSend for MachineHeartbeatAlert {
    fn to_telegram_message(&self) -> String {
        let last_response_time = self.last_response_time.to_utc().to_string();
        format!(
            "❗ *Machine Heartbeat Alert* ❗️\nFailed to receive heartbeat from machine\nID: `{}`\nLast heartbeat: `{}`",
            Self::escape_markdown_v2(&self.machine_id.0.to_string()),
            Self::escape_markdown_v2(&last_response_time)
        )
    }
}
