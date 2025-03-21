use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use ethers::abi::Address;
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

use crate::ClientId;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientHeartbeatAlert {
    pub client_id: ClientId,
    pub last_response_time: DateTime<Utc>,
}

pub struct DbClientHeartbeatAlert {
    pub client_id: Vec<u8>,
    pub last_response_time: NaiveDateTime,
}

impl From<DbClientHeartbeatAlert> for ClientHeartbeatAlert {
    fn from(value: DbClientHeartbeatAlert) -> Self {
        Self {
            client_id: ClientId(Address::from_slice(&value.client_id)),
            last_response_time: DateTime::<Utc>::from_naive_utc_and_offset(
                value.last_response_time,
                Utc,
            ),
        }
    }
}

impl ClientHeartbeatAlert {
    pub async fn get(pool: &PgPool, client_id: ClientId) -> Result<Option<Self>, DatabaseError> {
        let alert = sqlx::query_as!(
            DbClientHeartbeatAlert,
            "SELECT client_id, last_response_time FROM client_heartbeat WHERE client_id = $1",
            client_id.0.as_bytes()
        )
        .fetch_optional(pool)
        .await?;

        Ok(alert.map(|a| a.into()))
    }

    pub async fn insert(pool: &PgPool, alert: Self) -> Result<(), DatabaseError> {
        sqlx::query!(
            "INSERT INTO client_heartbeat (client_id, last_response_time) VALUES ($1, $2)",
            alert.client_id.0.as_bytes(),
            alert.last_response_time.naive_utc()
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &PgPool, client_id: ClientId) -> Result<(), DatabaseError> {
        sqlx::query!("DELETE FROM client_heartbeat WHERE client_id = $1", client_id.0.as_bytes())
            .execute(pool)
            .await?;

        Ok(())
    }
}

impl NotificationSend for ClientHeartbeatAlert {}

impl PagerDutySend for ClientHeartbeatAlert {
    fn to_pagerduty_message(&self) -> String {
        let last_response_time = self.last_response_time.to_utc().to_string();
        format!("Failed to recieve heartbeat from client. ID: {:?} | Last heartbeat sent at {last_response_time} UTC", self.client_id.0)
    }
}

impl From<ClientHeartbeatAlert> for Event {
    fn from(value: ClientHeartbeatAlert) -> Self {
        Self {
            routing_key: "".to_owned(),
            event_action: Action::Trigger,
            dedup_key: Uuid::new_v5(&Uuid::NAMESPACE_OID, value.client_id.0.as_bytes()),
            client: Some(value.client_id.0.to_string()),
            payload: Payload {
                severity: Severity::Error,
                source: "IvyNet".to_owned(),
                summary: value.to_pagerduty_message(),
                timestamp: chrono::Utc::now(),
                component: None,
            },
        }
    }
}

impl SendgridSend for ClientHeartbeatAlert {
    fn to_sendgrid_template_payload(self) -> SendgridParams {
        SendgridParams {
            email_template: EmailTemplate::NoClientHeartbeat,
            payload: HashMap::from([("client".to_string(), self.client_id.0.to_string())]),
        }
    }

    fn machine_id(&self) -> Option<Uuid> {
        None
    }

    fn error_type_msg(&self) -> String {
        format!("Failed to recieve heartbeat from client. ID: {:?}", self.client_id.0)
    }
}

impl TelegramSend for ClientHeartbeatAlert {
    fn to_telegram_message(&self) -> String {
        let last_response_time = self.last_response_time.to_utc().to_string();
        format!(
            "❗ *Client Heartbeat Alert* ❗️\nFailed to receive heartbeat from client\nID: `{}`\nLast heartbeat: `{}`",
            Self::escape_markdown_v2(&format!("{:?}", self.client_id.0)),
            Self::escape_markdown_v2(&last_response_time)
        )
    }
}
