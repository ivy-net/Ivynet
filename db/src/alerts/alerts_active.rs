use chrono::NaiveDateTime;
use ivynet_core::ethers::types::Address;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DatabaseError;

use super::{alerts_historical::HistoryAlert, AlertType};

pub struct NewAlert {
    alert_type: AlertType,
    machine_id: Uuid,
    organization_id: i64,
    client_id: Address,
    node_name: String,
    created_at: NaiveDateTime,
}

pub struct ActiveAlert {
    pub alert_id: i64,
    pub alert_type: AlertType,
    pub machine_id: Uuid,
    pub organization_id: i64,
    pub client_id: Address,
    pub node_name: String,
    pub created_at: NaiveDateTime,
    pub acknowledged_at: Option<NaiveDateTime>,
}

pub struct DbActiveAlert {
    alert_id: i64,
    alert_type: i64,
    machine_id: Uuid,
    organization_id: i64,
    client_id: Vec<u8>,
    node_name: String,
    created_at: NaiveDateTime,
    acknowledged_at: Option<NaiveDateTime>,
}

impl From<DbActiveAlert> for ActiveAlert {
    fn from(db_active_alert: DbActiveAlert) -> Self {
        ActiveAlert {
            alert_id: db_active_alert.alert_id,
            alert_type: db_active_alert.alert_type.into(),
            machine_id: db_active_alert.machine_id,
            organization_id: db_active_alert.organization_id,
            client_id: Address::from_slice(&db_active_alert.client_id),
            node_name: db_active_alert.node_name,
            created_at: db_active_alert.created_at,
            acknowledged_at: db_active_alert.acknowledged_at,
        }
    }
}

impl ActiveAlert {
    pub async fn record_new(pool: &PgPool, alert: &NewAlert) -> Result<(), DatabaseError> {
        sqlx::query!(
            r#"
            INSERT INTO alerts_active (
                alert_type,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6
            )
            "#,
            alert.alert_type as i16,
            alert.machine_id,
            alert.organization_id,
            alert.client_id.as_bytes().to_vec(),
            alert.node_name,
            alert.created_at,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn all_alerts_by_org(
        pool: &PgPool,
        organization_id: i64,
    ) -> Result<Vec<ActiveAlert>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbActiveAlert,
            r#"
            SELECT
                alert_id,
                alert_type,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at
            FROM alerts_active
            WHERE organization_id = $1
            "#,
            organization_id
        )
        .fetch_all(pool)
        .await?;

        Ok(alerts.into_iter().map(|n| n.into()).collect())
    }

    pub async fn all_alerts_by_machine(
        pool: &PgPool,
        machine_id: Uuid,
    ) -> Result<Vec<ActiveAlert>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbActiveAlert,
            r#"
            SELECT
                alert_id,
                alert_type,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at
            FROM alerts_active
            WHERE machine_id = $1
            "#,
            machine_id
        )
        .fetch_all(pool)
        .await?;

        Ok(alerts.into_iter().map(|n| n.into()).collect())
    }

    pub async fn acknowledge(pool: &PgPool, alert_id: i64) -> Result<(), DatabaseError> {
        sqlx::query!(
            r#"
            UPDATE alerts_active
            SET acknowledged_at = now()
            WHERE alert_id = $1
            "#,
            alert_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    // TODO: Code could be de-duplicated here if we can modify the function signature of
    // HistoryAlery::record_new to accept `&mut E: Exectuor` instead of `&PgPool`
    pub async fn resolve_alert(pool: &PgPool, alert_id: i64) -> Result<(), DatabaseError> {
        let mut tx = pool.begin().await?;
        let active_alert: ActiveAlert = sqlx::query_as!(
            DbActiveAlert,
            r#"
            SELECT
                alert_id,
                alert_type,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at
            FROM alerts_active
            WHERE alert_id = $1
            "#,
            alert_id
        )
        .fetch_one(pool) // Use the transaction if you want both queries to be part of the same transaction
        .await?
        .into();

        let history_alert: HistoryAlert = active_alert.into();

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
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                now()
            )
            "#,
            history_alert.alert_type as i16,
            history_alert.machine_id,
            history_alert.organization_id,
            history_alert.client_id.as_bytes().to_vec(),
            history_alert.node_name,
            history_alert.created_at,
            history_alert.acknowledged_at,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            DELETE FROM alerts_active
            WHERE alert_id = $1
            "#,
            alert_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}
