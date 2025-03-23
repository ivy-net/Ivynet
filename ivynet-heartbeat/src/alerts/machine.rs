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
    pub created_at: DateTime<Utc>,
    pub last_response_time: DateTime<Utc>,
}

pub struct DbMachineHeartbeatAlert {
    pub machine_id: Uuid,
    pub organization_id: i64,
    pub created_at: NaiveDateTime,
    pub last_response_time: NaiveDateTime,
}

impl From<DbMachineHeartbeatAlert> for MachineHeartbeatAlert {
    fn from(value: DbMachineHeartbeatAlert) -> Self {
        Self {
            machine_id: MachineId(value.machine_id),
            created_at: DateTime::<Utc>::from_naive_utc_and_offset(value.created_at, Utc),
            last_response_time: DateTime::<Utc>::from_naive_utc_and_offset(
                value.last_response_time,
                Utc,
            ),
        }
    }
}

impl DbMachineHeartbeatAlert {
    pub async fn get(pool: &PgPool, machine_id: Uuid) -> Result<Option<Self>, DatabaseError> {
        sqlx::query_as!(
            Self,
            "SELECT machine_id, organization_id, created_at, last_response_time FROM machine_heartbeat_alerts WHERE machine_id = $1",
            machine_id
        )
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }
}

impl MachineHeartbeatAlert {
    pub async fn get(pool: &PgPool, machine_id: MachineId) -> Result<Option<Self>, DatabaseError> {
        let alert = DbMachineHeartbeatAlert::get(pool, machine_id.0).await?;

        Ok(alert.map(|a| a.into()))
    }

    pub async fn insert(
        pool: &PgPool,
        alert: Self,
        organization_id: i64,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "INSERT INTO machine_heartbeat_alerts (machine_id, organization_id, created_at, last_response_time) VALUES ($1, $2, $3, $4)",
            alert.machine_id.0,
            organization_id,
            alert.created_at.naive_utc(),
            alert.last_response_time.naive_utc()
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &PgPool, machine_id: MachineId) -> Result<(), DatabaseError> {
        sqlx::query!("DELETE FROM machine_heartbeat_alerts WHERE machine_id = $1", machine_id.0)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn resolve(
        pool: &PgPool,
        machine_id: MachineId,
    ) -> Result<Option<i64>, DatabaseError> {
        let db_alert = DbMachineHeartbeatAlert::get(pool, machine_id.0).await?;

        if let Some(db_alert) = db_alert {
            let resolved_at = chrono::Utc::now().naive_utc();

            let result = sqlx::query!(
                r#"INSERT INTO machine_heartbeat_alerts_historical
                   (machine_id, organization_id, created_at, last_response_time, resolved_at)
                   VALUES ($1, $2, $3, $4, $5)
                   RETURNING id"#,
                machine_id.0,
                db_alert.organization_id,
                db_alert.created_at,
                db_alert.last_response_time,
                resolved_at
            )
            .fetch_one(pool)
            .await?;

            Self::delete(pool, machine_id).await?;

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
            DbMachineHeartbeatAlert,
            "SELECT machine_id, organization_id, created_at, last_response_time FROM machine_heartbeat_alerts WHERE organization_id = $1",
            organization_id
        )
        .fetch_all(pool)
        .await?;

        // Convert to domain objects
        let result: Vec<Self> = alerts.into_iter().map(|a| a.into()).collect();

        Ok(result)
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MachineHeartbeatAlertHistorical {
    pub id: i64,
    pub machine_id: MachineId,
    pub created_at: DateTime<Utc>,
    pub last_response_time: DateTime<Utc>,
    pub resolved_at: DateTime<Utc>,
}

pub struct DbMachineHeartbeatAlertHistorical {
    pub id: i64,
    pub machine_id: Uuid,
    pub created_at: NaiveDateTime,
    pub last_response_time: NaiveDateTime,
    pub resolved_at: NaiveDateTime,
}

impl From<DbMachineHeartbeatAlertHistorical> for MachineHeartbeatAlertHistorical {
    fn from(value: DbMachineHeartbeatAlertHistorical) -> Self {
        Self {
            id: value.id,
            machine_id: MachineId(value.machine_id),
            created_at: DateTime::<Utc>::from_naive_utc_and_offset(value.created_at, Utc),
            last_response_time: DateTime::<Utc>::from_naive_utc_and_offset(
                value.last_response_time,
                Utc,
            ),
            resolved_at: DateTime::<Utc>::from_naive_utc_and_offset(value.resolved_at, Utc),
        }
    }
}

impl MachineHeartbeatAlertHistorical {
    pub async fn get_by_organization_id(
        pool: &PgPool,
        organization_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbMachineHeartbeatAlertHistorical,
            r#"SELECT id, machine_id, created_at, last_response_time, resolved_at
               FROM machine_heartbeat_alerts_historical
               WHERE organization_id = $1
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#,
            organization_id,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        let result: Vec<Self> = alerts.into_iter().map(|a| a.into()).collect();

        Ok(result)
    }

    pub async fn get(
        pool: &PgPool,
        machine_id: MachineId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbMachineHeartbeatAlertHistorical,
            r#"SELECT id, machine_id, created_at, last_response_time, resolved_at
               FROM machine_heartbeat_alerts_historical
               WHERE machine_id = $1
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#,
            machine_id.0,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        let result: Vec<Self> = alerts.into_iter().map(|a| a.into()).collect();

        Ok(result)
    }
}

#[cfg(test)]
mod machine_heartbeat_alert_tests {
    use super::*;
    use chrono::Duration;
    use sqlx::PgPool;

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql")
    )]
    async fn test_machine_heartbeat_alert_lifecycle(pool: PgPool) {
        let now = Utc::now();
        let last_response = now - Duration::minutes(5);
        let machine_id = MachineId(Uuid::new_v4());
        let organization_id = 1i64; // From fixtures

        // Create an alert
        let alert = MachineHeartbeatAlert {
            machine_id: machine_id.clone(),
            created_at: now,
            last_response_time: last_response,
        };

        // Insert the alert
        MachineHeartbeatAlert::insert(&pool, alert.clone(), organization_id).await.unwrap();

        // Retrieve the alert
        let retrieved = MachineHeartbeatAlert::get(&pool, machine_id.clone()).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();

        // Verify alert data
        assert_eq!(retrieved.machine_id.0, machine_id.0);
        assert_eq!(retrieved.created_at.timestamp(), now.timestamp());
        assert_eq!(retrieved.last_response_time.timestamp(), last_response.timestamp());

        // Resolve the alert
        let historical_id =
            MachineHeartbeatAlert::resolve(&pool, machine_id.clone()).await.unwrap();
        assert!(historical_id.is_some());

        // Check that the alert is no longer in the active table
        let deleted_check = MachineHeartbeatAlert::get(&pool, machine_id.clone()).await.unwrap();
        assert!(deleted_check.is_none());

        // Check that it's in the historical table
        let historical =
            MachineHeartbeatAlertHistorical::get(&pool, machine_id.clone(), 10, 0).await.unwrap();

        assert_eq!(historical.len(), 1);
        assert_eq!(historical[0].machine_id.0, machine_id.0);
        assert!(historical[0].resolved_at > now); // Resolved timestamp should be after creation

        // Check organization-based query
        let org_historical =
            MachineHeartbeatAlertHistorical::get_by_organization_id(&pool, organization_id, 10, 0)
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
    async fn test_machine_heartbeat_pagination(pool: PgPool) {
        let organization_id = 1i64; // From fixtures

        // Create multiple machine alerts and immediately resolve them
        for _ in 0..5 {
            let now = Utc::now();
            let last_response = now - Duration::minutes(5);
            let machine_id = MachineId(Uuid::new_v4());

            let alert = MachineHeartbeatAlert {
                machine_id: machine_id.clone(),
                created_at: now,
                last_response_time: last_response,
            };

            // Insert and resolve each alert
            MachineHeartbeatAlert::insert(&pool, alert, organization_id).await.unwrap();
            MachineHeartbeatAlert::resolve(&pool, machine_id).await.unwrap();
        }

        // Test pagination with limit 2
        let page1 =
            MachineHeartbeatAlertHistorical::get_by_organization_id(&pool, organization_id, 2, 0)
                .await
                .unwrap();

        assert_eq!(page1.len(), 2);

        let page2 =
            MachineHeartbeatAlertHistorical::get_by_organization_id(&pool, organization_id, 2, 2)
                .await
                .unwrap();

        assert_eq!(page2.len(), 2);

        let page3 =
            MachineHeartbeatAlertHistorical::get_by_organization_id(&pool, organization_id, 2, 4)
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
    async fn test_resolve_nonexistent_machine_alert(pool: PgPool) {
        let machine_id = MachineId(Uuid::new_v4());

        // Try to resolve an alert that doesn't exist
        let result = MachineHeartbeatAlert::resolve(&pool, machine_id).await.unwrap();
        assert!(result.is_none());
    }

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql")
    )]
    async fn test_machine_heartbeat_get_by_organization(pool: PgPool) {
        let now = Utc::now();
        let last_response = now - Duration::minutes(5);
        let organization_id_1 = 1i64; // From fixtures
        let organization_id_2 = 2i64; // Different org

        // Create alerts for organization 1
        for _ in 0..3 {
            let machine_id = MachineId(Uuid::new_v4());

            let alert = MachineHeartbeatAlert {
                machine_id: machine_id.clone(),
                created_at: now,
                last_response_time: last_response,
            };

            MachineHeartbeatAlert::insert(&pool, alert, organization_id_1).await.unwrap();
        }

        // Create alerts for organization 2
        for _ in 0..2 {
            let machine_id = MachineId(Uuid::new_v4());

            let alert = MachineHeartbeatAlert {
                machine_id: machine_id.clone(),
                created_at: now,
                last_response_time: last_response,
            };

            MachineHeartbeatAlert::insert(&pool, alert, organization_id_2).await.unwrap();
        }

        // Verify organization 1 has 3 alerts
        let org1_alerts =
            MachineHeartbeatAlert::get_by_organization_id(&pool, organization_id_1).await.unwrap();
        assert_eq!(org1_alerts.len(), 3);

        // Verify organization 2 has 2 alerts
        let org2_alerts =
            MachineHeartbeatAlert::get_by_organization_id(&pool, organization_id_2).await.unwrap();
        assert_eq!(org2_alerts.len(), 2);

        // Verify organization 3 (which doesn't exist) has 0 alerts
        let org3_alerts = MachineHeartbeatAlert::get_by_organization_id(&pool, 3i64).await.unwrap();
        assert_eq!(org3_alerts.len(), 0);
    }
}
