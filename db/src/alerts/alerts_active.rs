use std::fmt::{self, Display, Formatter};

use chrono::NaiveDateTime;
use ivynet_core::ethers::types::Address;
use ivynet_notifications::NotificationType;
use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;

use super::{alert_handler::AlertType, alerts_historical::HistoryAlert};

#[derive(Debug, Clone, Serialize)]
pub struct NewAlert {
    pub alert_type: AlertType,
    pub machine_id: Uuid,
    pub node_name: String,
    pub created_at: NaiveDateTime,
}

impl NewAlert {
    pub fn new(machine_id: Uuid, alert_type: AlertType, node_name: String) -> Self {
        Self { alert_type, machine_id, node_name, created_at: chrono::Utc::now().naive_utc() }
    }

    pub fn generate_uuid(&self) -> Uuid {
        Uuid::new_v5(&Uuid::NAMESPACE_OID, self.to_string().as_bytes())
    }
}

/// Custom implementation of display which excludes the timestamp. Used primarily for UUID
/// generation.
impl Display for NewAlert {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?} {} {}", self.alert_type, self.machine_id, self.node_name)
    }
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct ActiveAlert {
    pub alert_id: Uuid,
    pub alert_type: NotificationType,
    pub machine_id: Uuid,
    pub organization_id: i64,
    pub client_id: Address,
    pub node_name: String,
    pub created_at: NaiveDateTime,
    pub acknowledged_at: Option<NaiveDateTime>,
}

pub struct DbActiveAlert {
    alert_id: Uuid,
    alert_type: i64,
    machine_id: Uuid,
    organization_id: i64,
    client_id: Vec<u8>,
    node_name: String,
    created_at: NaiveDateTime,
    acknowledged_at: Option<NaiveDateTime>,
    custom_data: Option<serde_json::Value>,
}

impl From<DbActiveAlert> for ActiveAlert {
    fn from(db_active_alert: DbActiveAlert) -> Self {
        let notification_type =
            NotificationType::try_from((db_active_alert.alert_type, db_active_alert.custom_data))
                .expect("Failed to convert alert type");
        ActiveAlert {
            alert_id: db_active_alert.alert_id,
            alert_type: notification_type,
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
    pub async fn get(pool: &PgPool, alert_id: Uuid) -> Result<Option<ActiveAlert>, DatabaseError> {
        let alert = sqlx::query_as!(
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
                acknowledged_at,
                custom_data
            FROM alerts_active
            WHERE alert_id = $1
            "#,
            alert_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(alert.map(|n| n.into()))
    }

    pub async fn get_all(pool: &PgPool) -> Result<Vec<ActiveAlert>, DatabaseError> {
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
                acknowledged_at,
                custom_data
            FROM alerts_active
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(alerts.into_iter().map(|n| n.into()).collect())
    }

    pub async fn insert_one(pool: &PgPool, alert: &NewAlert) -> Result<(), DatabaseError> {
        let alert_id = alert.generate_uuid();
        sqlx::query!(
            r#"
            INSERT INTO alerts_active (
                alert_id,
                alert_type,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at
            )
            SELECT
                $1,
                $2,
                m.machine_id,
                c.organization_id,
                m.client_id,
                $3,
                $4
            FROM machine m
            JOIN client c
              ON m.client_id = c.client_id
            WHERE m.machine_id = $5   -- lookup based on the provided machine_id
            "#,
            alert_id,
            alert.alert_type as i16,
            alert.node_name,
            alert.created_at,
            alert.machine_id,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn insert_many(pool: &PgPool, alerts: &[NewAlert]) -> Result<(), DatabaseError> {
        let mut tx = pool.begin().await?;
        for alert in alerts {
            let alert_id = alert.generate_uuid();
            sqlx::query!(
                r#"
                INSERT INTO alerts_active (
                    alert_id,
                    alert_type,
                    machine_id,
                    organization_id,
                    client_id,
                    node_name,
                    created_at
                )
                SELECT
                    $1,
                    $2,
                    m.machine_id,
                    c.organization_id,
                    m.client_id,
                    $3,
                    $4
                FROM machine m
                JOIN client c
                  ON m.client_id = c.client_id
                WHERE m.machine_id = $5   -- lookup based on the provided machine_id
                "#,
                alert_id,
                alert.alert_type as i16,
                alert.node_name,
                alert.created_at,
                alert.machine_id,
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
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

    pub async fn acknowledge(pool: &PgPool, alert_id: Uuid) -> Result<(), DatabaseError> {
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
    pub async fn resolve_alert(pool: &PgPool, alert_id: Uuid) -> Result<(), DatabaseError> {
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

        let alert_id = active_alert.alert_id;

        let history_alert: HistoryAlert = active_alert.into();

        sqlx::query!(
            r#"
            INSERT INTO alerts_historical (
                alert_id,
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
                $8,
                now()
            )
            "#,
            alert_id,
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
