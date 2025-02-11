use chrono::{Local, NaiveDateTime};
use ivynet_core::ethers::types::Address;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DatabaseError;

use super::{alert_actor::AlertType, alerts_active::ActiveAlert};

pub struct HistoryAlert {
    pub alert_id: i64,
    pub alert_type: AlertType,
    pub machine_id: Uuid,
    pub organization_id: i64,
    pub client_id: Address,
    pub node_name: String,
    pub created_at: NaiveDateTime,
    pub acknowledged_at: Option<NaiveDateTime>,
    pub resolved_at: NaiveDateTime,
}

pub struct DbHistoryAlert {
    alert_id: i64,
    alert_type: i64,
    machine_id: Uuid,
    organization_id: i64,
    client_id: Vec<u8>,
    node_name: String,
    created_at: NaiveDateTime,
    acknowledged_at: Option<NaiveDateTime>,
    resolved_at: NaiveDateTime,
}

impl From<HistoryAlert> for DbHistoryAlert {
    fn from(value: HistoryAlert) -> Self {
        Self {
            alert_id: value.alert_id,
            alert_type: value.alert_type as i64,
            machine_id: value.machine_id,
            organization_id: value.organization_id,
            client_id: value.client_id.as_bytes().to_vec(),
            node_name: value.node_name,
            created_at: value.created_at,
            acknowledged_at: value.acknowledged_at,
            resolved_at: value.resolved_at,
        }
    }
}

impl From<DbHistoryAlert> for HistoryAlert {
    fn from(value: DbHistoryAlert) -> Self {
        Self {
            alert_id: value.alert_id,
            alert_type: value.alert_type.into(),
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

impl From<ActiveAlert> for HistoryAlert {
    fn from(value: ActiveAlert) -> Self {
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

impl HistoryAlert {
    pub async fn get(pool: &PgPool, alert_id: i64) -> Result<Option<HistoryAlert>, DatabaseError> {
        let db_history_alert = sqlx::query_as!(
            DbHistoryAlert,
            r#"
            SELECT
                alert_id,
                alert_type,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                resolved_at
            FROM
                alerts_historical
            WHERE
                alert_id = $1
            "#,
            alert_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(db_history_alert.map(|a| a.into()))
    }

    #[cfg(test)]
    pub async fn get_all(pool: &PgPool) -> Result<Vec<HistoryAlert>, DatabaseError> {
        let db_history_alerts = sqlx::query_as!(
            DbHistoryAlert,
            r#"
            SELECT
                alert_id,
                alert_type,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                resolved_at
            FROM
                alerts_historical
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(db_history_alerts.into_iter().map(|a| a.into()).collect())
    }

    pub async fn record_new(pool: &PgPool, alert: &HistoryAlert) -> Result<(), DatabaseError> {
        sqlx::query!(
            r#"
            INSERT INTO alerts_historical (
                alert_type,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                resolved_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            alert.alert_type as i64,
            alert.machine_id,
            alert.organization_id,
            alert.client_id.as_bytes().to_vec(),
            alert.node_name,
            alert.created_at,
            alert.acknowledged_at,
            alert.resolved_at,
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
    ) -> Result<Vec<HistoryAlert>, DatabaseError> {
        let db_history_alerts = sqlx::query_as!(
            DbHistoryAlert,
            r#"
            SELECT
                alert_id,
                alert_type,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                resolved_at
            FROM
                alerts_historical
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
    ) -> Result<Vec<HistoryAlert>, DatabaseError> {
        let db_history_alerts = sqlx::query_as!(
            DbHistoryAlert,
            r#"
            SELECT
                alert_id,
                alert_type,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                resolved_at
            FROM
                alerts_historical
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
    ) -> Result<Vec<HistoryAlert>, DatabaseError> {
        let db_history_alerts = sqlx::query_as!(
            DbHistoryAlert,
            r#"
            SELECT
                alert_id,
                alert_type,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                resolved_at
            FROM
                alerts_historical
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
