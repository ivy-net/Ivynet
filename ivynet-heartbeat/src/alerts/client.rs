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
use utoipa::ToSchema;
use uuid::Uuid;

use crate::ClientId;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ClientHeartbeatAlert {
    #[schema(value_type = String)]
    pub client_id: ClientId,
    #[schema(value_type = String)]
    pub created_at: DateTime<Utc>,
    #[schema(value_type = String)]
    pub last_response_time: DateTime<Utc>,
}

pub struct DbClientHeartbeatAlert {
    pub client_id: Vec<u8>,
    pub organization_id: i64,
    pub created_at: NaiveDateTime,
    pub last_response_time: NaiveDateTime,
}

impl From<DbClientHeartbeatAlert> for ClientHeartbeatAlert {
    fn from(value: DbClientHeartbeatAlert) -> Self {
        Self {
            client_id: ClientId(Address::from_slice(&value.client_id)),
            created_at: DateTime::<Utc>::from_naive_utc_and_offset(value.created_at, Utc),
            last_response_time: DateTime::<Utc>::from_naive_utc_and_offset(
                value.last_response_time,
                Utc,
            ),
        }
    }
}

impl DbClientHeartbeatAlert {
    pub async fn get(pool: &PgPool, client_id: &[u8]) -> Result<Option<Self>, DatabaseError> {
        sqlx::query_as!(
            Self,
            "SELECT client_id, organization_id, created_at, last_response_time FROM client_heartbeat_alerts WHERE client_id = $1",
            client_id
        )
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }
}

impl ClientHeartbeatAlert {
    pub async fn get(pool: &PgPool, client_id: ClientId) -> Result<Option<Self>, DatabaseError> {
        let alert = DbClientHeartbeatAlert::get(pool, client_id.0.as_bytes()).await?;

        Ok(alert.map(|a| a.into()))
    }

    pub async fn insert(
        pool: &PgPool,
        alert: Self,
        organization_id: i64,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "INSERT INTO client_heartbeat_alerts (client_id, organization_id, created_at, last_response_time) VALUES ($1, $2, $3, $4)",
            alert.client_id.0.as_bytes(),
            organization_id,
            alert.created_at.naive_utc(),
            alert.last_response_time.naive_utc()
        )
.execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &PgPool, client_id: ClientId) -> Result<(), DatabaseError> {
        sqlx::query!(
            "DELETE FROM client_heartbeat_alerts WHERE client_id = $1",
            client_id.0.as_bytes()
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn resolve(pool: &PgPool, client_id: ClientId) -> Result<Option<i64>, DatabaseError> {
        let db_alert = DbClientHeartbeatAlert::get(pool, client_id.0.as_bytes()).await?;

        if let Some(db_alert) = db_alert {
            let resolved_at = chrono::Utc::now().naive_utc();

            let result = sqlx::query!(
                r#"INSERT INTO client_heartbeat_alerts_historical
                   (client_id, organization_id, created_at, last_response_time, resolved_at)
                   VALUES ($1, $2, $3, $4, $5)
                   RETURNING id"#,
                client_id.0.as_bytes(),
                db_alert.organization_id,
                db_alert.created_at,
                db_alert.last_response_time,
                resolved_at
            )
            .fetch_one(pool)
            .await?;

            Self::delete(pool, client_id).await?;

            Ok(Some(result.id))
        } else {
            Ok(None)
        }
    }

    pub async fn get_by_organization_id(
        pool: &PgPool,
        organization_id: i64,
    ) -> Result<Vec<Self>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbClientHeartbeatAlert,
            "SELECT client_id, organization_id, created_at, last_response_time FROM client_heartbeat_alerts WHERE organization_id = $1",
            organization_id
        )
        .fetch_all(pool)
        .await?;

        // Convert to domain objects
        let result: Vec<Self> = alerts.into_iter().map(|a| a.into()).collect();

        Ok(result)
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

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ClientHeartbeatAlertHistorical {
    pub id: i64,
    #[schema(value_type = String)]
    pub client_id: ClientId,
    #[schema(value_type = String)]
    pub created_at: DateTime<Utc>,
    #[schema(value_type = String)]
    pub last_response_time: DateTime<Utc>,
    #[schema(value_type = String)]
    pub resolved_at: DateTime<Utc>,
}

pub struct DbClientHeartbeatAlertHistorical {
    pub id: i64,
    pub client_id: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub last_response_time: NaiveDateTime,
    pub resolved_at: NaiveDateTime,
}

impl From<DbClientHeartbeatAlertHistorical> for ClientHeartbeatAlertHistorical {
    fn from(value: DbClientHeartbeatAlertHistorical) -> Self {
        Self {
            id: value.id,
            client_id: ClientId(Address::from_slice(&value.client_id)),
            created_at: DateTime::<Utc>::from_naive_utc_and_offset(value.created_at, Utc),
            last_response_time: DateTime::<Utc>::from_naive_utc_and_offset(
                value.last_response_time,
                Utc,
            ),
            resolved_at: DateTime::<Utc>::from_naive_utc_and_offset(value.resolved_at, Utc),
        }
    }
}

impl ClientHeartbeatAlertHistorical {
    pub async fn get_by_organization_id(
        pool: &PgPool,
        organization_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbClientHeartbeatAlertHistorical,
            r#"SELECT id, client_id, created_at, last_response_time, resolved_at
               FROM client_heartbeat_alerts_historical
               WHERE organization_id = $1
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#,
            organization_id,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        // Convert to domain objects
        let result: Vec<Self> = alerts.into_iter().map(|a| a.into()).collect();

        Ok(result)
    }

    pub async fn get(
        pool: &PgPool,
        client_id: ClientId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbClientHeartbeatAlertHistorical,
            r#"SELECT id, client_id, created_at, last_response_time, resolved_at
               FROM client_heartbeat_alerts_historical
               WHERE client_id = $1
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#,
            client_id.0.as_bytes(),
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        // Convert to domain objects
        let result: Vec<Self> = alerts.into_iter().map(|a| a.into()).collect();

        Ok(result)
    }
}

#[cfg(test)]
mod client_heartbeat_alert_tests {
    use super::*;
    use chrono::Duration;
    use ethers::abi::Address;
    use sqlx::PgPool;

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql")
    )]
    async fn test_client_heartbeat_alert_lifecycle(pool: PgPool) {
        let now = Utc::now();
        let last_response = now - Duration::minutes(5);
        let client_id = ClientId("0101010101010101010101010101010101010101".parse().unwrap());
        let organization_id = 1i64; // From fixtures

        // Create an alert
        let alert = ClientHeartbeatAlert {
            client_id: client_id.clone(),
            created_at: now,
            last_response_time: last_response,
        };

        // Insert the alert
        ClientHeartbeatAlert::insert(&pool, alert.clone(), organization_id).await.unwrap();

        // Retrieve the alert
        let retrieved = ClientHeartbeatAlert::get(&pool, client_id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();

        // Verify alert data
        assert_eq!(retrieved.client_id.0, client_id.0);
        assert_eq!(retrieved.created_at.timestamp(), now.timestamp());
        assert_eq!(retrieved.last_response_time.timestamp(), last_response.timestamp());

        // Resolve the alert
        let historical_id = ClientHeartbeatAlert::resolve(&pool, client_id).await.unwrap();
        assert!(historical_id.is_some());

        // Check that the alert is no longer in the active table
        let deleted_check = ClientHeartbeatAlert::get(&pool, client_id).await.unwrap();
        assert!(deleted_check.is_none());

        // Check that it's in the historical table
        let historical =
            ClientHeartbeatAlertHistorical::get(&pool, client_id, 10, 0).await.unwrap();

        assert_eq!(historical.len(), 1);
        assert_eq!(historical[0].client_id.0, client_id.0);
        assert!(historical[0].resolved_at > now); // Resolved timestamp should be after creation

        // Check organization-based query
        let org_historical =
            ClientHeartbeatAlertHistorical::get_by_organization_id(&pool, organization_id, 10, 0)
                .await
                .unwrap();

        assert_eq!(org_historical.len(), 1);
        assert_eq!(org_historical[0].id, historical[0].id);
    }

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql")
    )]
    async fn test_client_heartbeat_pagination(pool: PgPool) {
        let organization_id = 1i64; // From fixtures

        // Create multiple client alerts and immediately resolve them
        for i in 0..5 {
            let now = Utc::now();
            let last_response = now - Duration::minutes(5);
            // Create different client IDs by using the loop index
            let mut addr_bytes = [0u8; 20];
            addr_bytes[19] = i as u8;

            let client_id = ClientId(Address::from_slice(&addr_bytes));

            let alert = ClientHeartbeatAlert {
                client_id,
                created_at: now,
                last_response_time: last_response,
            };

            // Insert and resolve each alert
            ClientHeartbeatAlert::insert(&pool, alert, organization_id).await.unwrap();
            ClientHeartbeatAlert::resolve(&pool, client_id).await.unwrap();
        }

        // Test pagination with limit 2
        let page1 =
            ClientHeartbeatAlertHistorical::get_by_organization_id(&pool, organization_id, 2, 0)
                .await
                .unwrap();

        assert_eq!(page1.len(), 2);

        let page2 =
            ClientHeartbeatAlertHistorical::get_by_organization_id(&pool, organization_id, 2, 2)
                .await
                .unwrap();

        assert_eq!(page2.len(), 2);

        let page3 =
            ClientHeartbeatAlertHistorical::get_by_organization_id(&pool, organization_id, 2, 4)
                .await
                .unwrap();

        assert_eq!(page3.len(), 1);

        // Verify we have different records on different pages
        assert_ne!(page1[0].id, page2[0].id);
        assert_ne!(page1[1].id, page2[1].id);
    }

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql")
    )]
    async fn test_resolve_nonexistent_client_alert(pool: PgPool) {
        let client_id = ClientId("2222222222222222222222222222222222222222".parse().unwrap());

        // Try to resolve an alert that doesn't exist
        let result = ClientHeartbeatAlert::resolve(&pool, client_id).await.unwrap();
        assert!(result.is_none());
    }

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql")
    )]
    async fn test_client_heartbeat_get_by_organization(pool: PgPool) {
        let now = Utc::now();
        let last_response = now - Duration::minutes(5);
        let organization_id_1 = 1i64; // From fixtures
        let organization_id_2 = 2i64; // Different org

        // Create alerts for organization 1
        for i in 0..3 {
            let mut addr_bytes = [0u8; 20];
            addr_bytes[19] = i as u8;

            let client_id = ClientId(Address::from_slice(&addr_bytes));

            let alert = ClientHeartbeatAlert {
                client_id,
                created_at: now,
                last_response_time: last_response,
            };

            ClientHeartbeatAlert::insert(&pool, alert, organization_id_1).await.unwrap();
        }

        // Create alerts for organization 2
        for i in 3..5 {
            let mut addr_bytes = [0u8; 20];
            addr_bytes[19] = i as u8;

            let client_id = ClientId(Address::from_slice(&addr_bytes));

            let alert = ClientHeartbeatAlert {
                client_id,
                created_at: now,
                last_response_time: last_response,
            };

            ClientHeartbeatAlert::insert(&pool, alert, organization_id_2).await.unwrap();
        }

        // Verify organization 1 has 3 alerts
        let org1_alerts =
            ClientHeartbeatAlert::get_by_organization_id(&pool, organization_id_1).await.unwrap();
        assert_eq!(org1_alerts.len(), 3);

        // Verify organization 2 has 2 alerts
        let org2_alerts =
            ClientHeartbeatAlert::get_by_organization_id(&pool, organization_id_2).await.unwrap();
        assert_eq!(org2_alerts.len(), 2);

        // Verify organization 3 (which doesn't exist) has 0 alerts
        let org3_alerts = ClientHeartbeatAlert::get_by_organization_id(&pool, 3i64).await.unwrap();
        assert_eq!(org3_alerts.len(), 0);
    }
}
