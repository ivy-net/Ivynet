use chrono::{Local, NaiveDateTime};
use ivynet_alerts::Alert;
use ivynet_error::ethers::types::Address;
use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;

use super::alerts_active::NodeActiveAlert;

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct NodeHistoryAlert {
    pub alert_id: Uuid,
    pub alert_type: Alert,
    pub machine_id: Uuid,
    pub organization_id: i64,
    pub client_id: Address,
    pub node_name: String,
    pub created_at: NaiveDateTime,
    pub acknowledged_at: Option<NaiveDateTime>,
    pub resolved_at: NaiveDateTime,
}

pub struct DbNodeHistoryAlert {
    alert_id: Uuid,
    machine_id: Uuid,
    organization_id: i64,
    client_id: Vec<u8>,
    node_name: String,
    created_at: NaiveDateTime,
    acknowledged_at: Option<NaiveDateTime>,
    resolved_at: NaiveDateTime,
    alert_data: serde_json::Value,
}

impl From<NodeHistoryAlert> for DbNodeHistoryAlert {
    fn from(value: NodeHistoryAlert) -> Self {
        Self {
            alert_id: value.alert_id,
            machine_id: value.machine_id,
            organization_id: value.organization_id,
            client_id: value.client_id.as_bytes().to_vec(),
            node_name: value.node_name,
            created_at: value.created_at,
            acknowledged_at: value.acknowledged_at,
            resolved_at: value.resolved_at,
            alert_data: serde_json::json!(value.alert_type),
        }
    }
}

impl From<DbNodeHistoryAlert> for NodeHistoryAlert {
    fn from(value: DbNodeHistoryAlert) -> Self {
        Self {
            alert_id: value.alert_id,
            alert_type: serde_json::from_value(value.alert_data)
                .expect("Could not deserialize alert type"),
            machine_id: value.machine_id,
            organization_id: value.organization_id,
            client_id: Address::from_slice(&value.client_id),
            node_name: value.node_name,
            created_at: value.created_at,
            acknowledged_at: value.acknowledged_at,
            resolved_at: value.resolved_at,
        }
    }
}

impl From<NodeActiveAlert> for NodeHistoryAlert {
    fn from(value: NodeActiveAlert) -> Self {
        let now = Local::now().naive_utc();
        Self {
            alert_id: value.alert_id,
            alert_type: value.alert_type,
            machine_id: value.machine_id,
            organization_id: value.organization_id,
            client_id: value.client_id,
            node_name: value.node_name,
            created_at: value.created_at,
            acknowledged_at: value.acknowledged_at,
            resolved_at: now,
        }
    }
}

impl NodeHistoryAlert {
    pub async fn get(
        pool: &PgPool,
        alert_id: Uuid,
    ) -> Result<Option<NodeHistoryAlert>, DatabaseError> {
        let db_history_alert = sqlx::query_as!(
            DbNodeHistoryAlert,
            r#"
            SELECT
                alert_id,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                resolved_at,
                alert_data
            FROM
                node_alerts_historical
            WHERE
                alert_id = $1
            "#,
            alert_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(db_history_alert.map(|a| a.into()))
    }

    pub async fn get_all(pool: &PgPool) -> Result<Vec<NodeHistoryAlert>, DatabaseError> {
        let db_history_alerts = sqlx::query_as!(
            DbNodeHistoryAlert,
            r#"
            SELECT
                alert_id,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                resolved_at,
                alert_data
            FROM
                node_alerts_historical
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(db_history_alerts.into_iter().map(|a| a.into()).collect())
    }

    pub async fn record_new(pool: &PgPool, alert: &NodeHistoryAlert) -> Result<(), DatabaseError> {
        let alert_data = serde_json::json!(alert.alert_type);
        sqlx::query!(
            r#"
            INSERT INTO node_alerts_historical (
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                resolved_at,
                alert_data
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            alert.machine_id,
            alert.organization_id,
            alert.client_id.as_bytes().to_vec(),
            alert.node_name,
            alert.created_at,
            alert.acknowledged_at,
            alert.resolved_at,
            alert_data
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn alerts_by_org_between(
        pool: &PgPool,
        organization_id: i64,
        from: NaiveDateTime,
        to: NaiveDateTime,
    ) -> Result<Vec<NodeHistoryAlert>, DatabaseError> {
        let db_history_alerts = sqlx::query_as!(
            DbNodeHistoryAlert,
            r#"
            SELECT
                alert_id,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                resolved_at,
                alert_data
            FROM
                node_alerts_historical
            WHERE
                organization_id = $1
                AND created_at >= $2
                AND created_at <= $3
            "#,
            organization_id,
            from,
            to
        )
        .fetch_all(pool)
        .await?;

        Ok(db_history_alerts.into_iter().map(|a| a.into()).collect())
    }

    pub async fn all_alerts_by_org(
        pool: &PgPool,
        organization_id: i64,
    ) -> Result<Vec<NodeHistoryAlert>, DatabaseError> {
        let db_history_alerts = sqlx::query_as!(
            DbNodeHistoryAlert,
            r#"
            SELECT
                alert_id,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                resolved_at,
                alert_data
            FROM
                node_alerts_historical
            WHERE
                organization_id = $1
            "#,
            organization_id
        )
        .fetch_all(pool)
        .await?;

        Ok(db_history_alerts.into_iter().map(|a| a.into()).collect())
    }

    pub async fn all_alerts_by_machine(
        pool: &PgPool,
        machine_id: Uuid,
    ) -> Result<Vec<NodeHistoryAlert>, DatabaseError> {
        let db_history_alerts = sqlx::query_as!(
            DbNodeHistoryAlert,
            r#"
            SELECT
                alert_id,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                resolved_at,
                alert_data
            FROM
                node_alerts_historical
            WHERE
                machine_id = $1
            "#,
            machine_id
        )
        .fetch_all(pool)
        .await?;

        Ok(db_history_alerts.into_iter().map(|a| a.into()).collect())
    }
}
