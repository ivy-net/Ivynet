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
use utoipa::ToSchema;
use uuid::Uuid;

use crate::NodeId;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct NodeHeartbeatAlert {
    pub node_id: NodeId,
    #[schema(value_type = String)]
    pub created_at: DateTime<Utc>,
    #[schema(value_type = String)]
    pub last_response_time: DateTime<Utc>,
}

pub struct DbNodeHeartbeatAlert {
    pub node_id: String,
    pub organization_id: i64,
    pub created_at: NaiveDateTime,
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
                created_at: DateTime::<Utc>::from_naive_utc_and_offset(value.created_at, Utc),
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

impl DbNodeHeartbeatAlert {
    pub async fn get(pool: &PgPool, node_id: &str) -> Result<Option<Self>, DatabaseError> {
        sqlx::query_as!(
            Self,
            "SELECT node_id, organization_id, created_at, last_response_time FROM node_heartbeat_alerts WHERE node_id = $1",
            node_id
        )
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
    }
}

impl NodeHeartbeatAlert {
    pub async fn get(pool: &PgPool, node_id: NodeId) -> Result<Option<Self>, DatabaseError> {
        let node_id_str = format!("{}:{}", node_id.machine, node_id.name);
        let alert = DbNodeHeartbeatAlert::get(pool, &node_id_str).await?;

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
            "INSERT INTO node_heartbeat_alerts (node_id, organization_id, created_at, last_response_time) VALUES ($1, $2, $3, $4)",
            format!("{}:{}", alert.node_id.machine, alert.node_id.name),
            organization_id,
            alert.created_at.naive_utc(),
            alert.last_response_time.naive_utc()
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &PgPool, node_id: NodeId) -> Result<(), DatabaseError> {
        sqlx::query!("DELETE FROM node_heartbeat_alerts WHERE node_id = $1", node_id.to_string())
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn resolve(pool: &PgPool, node_id: NodeId) -> Result<Option<i64>, DatabaseError> {
        let node_id_str = format!("{}:{}", node_id.machine, node_id.name);
        let db_alert = DbNodeHeartbeatAlert::get(pool, &node_id_str).await?;

        if let Some(db_alert) = db_alert {
            let resolved_at = chrono::Utc::now().naive_utc();

            let result = sqlx::query!(
                r#"INSERT INTO node_heartbeat_alerts_historical
                   (node_id, organization_id, created_at, last_response_time, resolved_at)
                   VALUES ($1, $2, $3, $4, $5)
                   RETURNING id"#,
                node_id_str,
                db_alert.organization_id,
                db_alert.created_at,
                db_alert.last_response_time,
                resolved_at
            )
            .fetch_one(pool)
            .await?;

            Self::delete(pool, node_id).await?;

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
            DbNodeHeartbeatAlert,
            "SELECT node_id, organization_id, created_at, last_response_time FROM node_heartbeat_alerts WHERE organization_id = $1",
            organization_id
        )
        .fetch_all(pool)
        .await?;

        // Convert to domain objects
        let mut result = Vec::with_capacity(alerts.len());
        for alert in alerts {
            result.push(alert.try_into()?);
        }

        Ok(result)
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

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct NodeHeartbeatAlertHistorical {
    pub id: i64,
    pub node_id: NodeId,
    pub created_at: DateTime<Utc>,
    pub last_response_time: DateTime<Utc>,
    pub resolved_at: DateTime<Utc>,
}

pub struct DbNodeHeartbeatAlertHistorical {
    pub id: i64,
    pub node_id: String,
    pub created_at: NaiveDateTime,
    pub last_response_time: NaiveDateTime,
    pub resolved_at: NaiveDateTime,
}

impl TryFrom<DbNodeHeartbeatAlertHistorical> for NodeHeartbeatAlertHistorical {
    type Error = DatabaseError;

    fn try_from(value: DbNodeHeartbeatAlertHistorical) -> Result<Self, Self::Error> {
        let node_id_raw = value.node_id.clone();
        let parts = value.node_id.split_once(':');
        if let Some((machine, name)) = parts {
            Ok(Self {
                id: value.id,
                node_id: NodeId {
                    machine: Uuid::parse_str(machine)
                        .map_err(|_| DatabaseError::NodeIdParseError(node_id_raw))?,
                    name: name.to_string(),
                },
                created_at: DateTime::<Utc>::from_naive_utc_and_offset(value.created_at, Utc),
                last_response_time: DateTime::<Utc>::from_naive_utc_and_offset(
                    value.last_response_time,
                    Utc,
                ),
                resolved_at: DateTime::<Utc>::from_naive_utc_and_offset(value.resolved_at, Utc),
            })
        } else {
            Err(DatabaseError::NodeIdParseError(node_id_raw))
        }
    }
}

impl NodeHeartbeatAlertHistorical {
    pub async fn get_by_organization_id(
        pool: &PgPool,
        organization_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbNodeHeartbeatAlertHistorical,
            r#"SELECT id, node_id, created_at, last_response_time, resolved_at
               FROM node_heartbeat_alerts_historical
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
        let mut result = Vec::with_capacity(alerts.len());
        for alert in alerts {
            result.push(alert.try_into()?);
        }

        Ok(result)
    }

    pub async fn get(
        pool: &PgPool,
        node_id: NodeId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbNodeHeartbeatAlertHistorical,
            r#"SELECT id, node_id, created_at, last_response_time, resolved_at
               FROM node_heartbeat_alerts_historical
               WHERE node_id = $1
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#,
            format!("{}:{}", node_id.machine, node_id.name),
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        // Convert to domain objects
        let mut result = Vec::with_capacity(alerts.len());
        for alert in alerts {
            result.push(alert.try_into()?);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod node_heartbeat_alert_tests {
    use super::*;
    use chrono::Duration;
    use sqlx::PgPool;
    use uuid::Uuid;

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql")
    )]
    async fn test_node_heartbeat_alert_lifecycle(pool: PgPool) {
        let now = Utc::now();
        let last_response = now - Duration::minutes(5);
        let node_id = NodeId { machine: Uuid::new_v4(), name: "test-node".to_string() };
        let organization_id = 1i64; // From fixtures

        // Create an alert
        let alert = NodeHeartbeatAlert {
            node_id: node_id.clone(),
            created_at: now,
            last_response_time: last_response,
        };

        // Insert the alert
        NodeHeartbeatAlert::insert(&pool, alert.clone(), organization_id).await.unwrap();

        // Retrieve the alert
        let retrieved = NodeHeartbeatAlert::get(&pool, node_id.clone()).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();

        // Verify alert data
        assert_eq!(retrieved.node_id.machine, node_id.machine);
        assert_eq!(retrieved.node_id.name, node_id.name);
        assert_eq!(retrieved.created_at.timestamp(), now.timestamp());
        assert_eq!(retrieved.last_response_time.timestamp(), last_response.timestamp());

        // Resolve the alert
        let historical_id = NodeHeartbeatAlert::resolve(&pool, node_id.clone()).await.unwrap();
        assert!(historical_id.is_some());

        // Check that the alert is no longer in the active table
        let deleted_check = NodeHeartbeatAlert::get(&pool, node_id.clone()).await.unwrap();
        assert!(deleted_check.is_none());

        // Check that it's in the historical table
        let historical =
            NodeHeartbeatAlertHistorical::get(&pool, node_id.clone(), 10, 0).await.unwrap();

        assert_eq!(historical.len(), 1);
        assert_eq!(historical[0].node_id.machine, node_id.machine);
        assert_eq!(historical[0].node_id.name, node_id.name);
        assert!(historical[0].resolved_at > now); // Resolved timestamp should be after creation

        // Check organization-based query
        let org_historical =
            NodeHeartbeatAlertHistorical::get_by_organization_id(&pool, organization_id, 10, 0)
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
    async fn test_node_heartbeat_pagination(pool: PgPool) {
        let organization_id = 1i64; // From fixtures
        let machine_id = Uuid::new_v4();

        // Create multiple node alerts and immediately resolve them
        for i in 0..5 {
            let now = Utc::now();
            let last_response = now - Duration::minutes(5);
            let node_id = NodeId { machine: machine_id, name: format!("test-node-{}", i) };

            let alert = NodeHeartbeatAlert {
                node_id: node_id.clone(),
                created_at: now,
                last_response_time: last_response,
            };

            // Insert and resolve each alert
            NodeHeartbeatAlert::insert(&pool, alert, organization_id).await.unwrap();
            NodeHeartbeatAlert::resolve(&pool, node_id).await.unwrap();
        }

        // Test pagination with limit 2
        let page1 =
            NodeHeartbeatAlertHistorical::get_by_organization_id(&pool, organization_id, 2, 0)
                .await
                .unwrap();

        assert_eq!(page1.len(), 2);

        let page2 =
            NodeHeartbeatAlertHistorical::get_by_organization_id(&pool, organization_id, 2, 2)
                .await
                .unwrap();

        assert_eq!(page2.len(), 2);

        let page3 =
            NodeHeartbeatAlertHistorical::get_by_organization_id(&pool, organization_id, 2, 4)
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
    async fn test_resolve_nonexistent_alert(pool: PgPool) {
        let node_id = NodeId { machine: Uuid::new_v4(), name: "nonexistent-node".to_string() };

        // Try to resolve an alert that doesn't exist
        let result = NodeHeartbeatAlert::resolve(&pool, node_id).await.unwrap();
        assert!(result.is_none());
    }

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql")
    )]
    async fn test_node_heartbeat_get_by_organization(pool: PgPool) {
        let now = Utc::now();
        let last_response = now - Duration::minutes(5);
        let organization_id_1 = 1i64; // From fixtures
        let organization_id_2 = 2i64; // Different org

        let machine_id_1 = Uuid::new_v4();
        let machine_id_2 = Uuid::new_v4();

        // Create alerts for organization 1
        for i in 0..3 {
            let node_id = NodeId { machine: machine_id_1, name: format!("test-node-org1-{}", i) };

            let alert = NodeHeartbeatAlert {
                node_id: node_id.clone(),
                created_at: now,
                last_response_time: last_response,
            };

            NodeHeartbeatAlert::insert(&pool, alert, organization_id_1).await.unwrap();
        }

        // Create alerts for organization 2
        for i in 0..2 {
            let node_id = NodeId { machine: machine_id_2, name: format!("test-node-org2-{}", i) };

            let alert = NodeHeartbeatAlert {
                node_id: node_id.clone(),
                created_at: now,
                last_response_time: last_response,
            };

            NodeHeartbeatAlert::insert(&pool, alert, organization_id_2).await.unwrap();
        }

        // Verify organization 1 has 3 alerts
        let org1_alerts =
            NodeHeartbeatAlert::get_by_organization_id(&pool, organization_id_1).await.unwrap();
        assert_eq!(org1_alerts.len(), 3);

        // Verify organization 2 has 2 alerts
        let org2_alerts =
            NodeHeartbeatAlert::get_by_organization_id(&pool, organization_id_2).await.unwrap();
        assert_eq!(org2_alerts.len(), 2);

        // Verify organization 3 (which doesn't exist) has 0 alerts
        let org3_alerts = NodeHeartbeatAlert::get_by_organization_id(&pool, 3i64).await.unwrap();
        assert_eq!(org3_alerts.len(), 0);
    }
}
