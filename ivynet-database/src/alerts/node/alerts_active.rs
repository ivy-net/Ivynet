use std::fmt::{self, Display, Formatter};

use chrono::NaiveDateTime;
use ivynet_alerts::{Alert, SendState};
use ivynet_error::ethers::types::Address;
use ivynet_notifications::Channel;
use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{error::DatabaseError, Avs};

use super::alerts_historical::NodeHistoryAlert;

#[derive(Debug, Clone, Serialize)]
pub struct NewNodeAlert {
    pub id: Uuid,
    pub alert_type: Alert,
    pub machine_id: Uuid,
    pub node_name: String,
    pub created_at: NaiveDateTime,
    pub telegram_send: SendState,
    pub sendgrid_send: SendState,
    pub pagerduty_send: SendState,
}

impl NewNodeAlert {
    pub fn new(machine_id: Uuid, alert_type: Alert, node_name: String) -> Self {
        let alert_id = alert_type.uuid_seed();
        let str_rep = format!("{}-{}-{}", alert_id, machine_id, node_name);
        let id = Uuid::new_v5(&Uuid::NAMESPACE_OID, str_rep.as_bytes());
        Self {
            id,
            alert_type,
            machine_id,
            node_name,
            created_at: chrono::Utc::now().naive_utc(),
            telegram_send: SendState::NoSend,
            sendgrid_send: SendState::NoSend,
            pagerduty_send: SendState::NoSend,
        }
    }

    pub fn set_send_state(&mut self, send_type: Channel, state: SendState) {
        match send_type {
            Channel::Telegram(_) => self.telegram_send = state,
            Channel::Email(_) => self.sendgrid_send = state,
            Channel::PagerDuty(_) => self.pagerduty_send = state,
        }
    }

    /// Get the inner uint representation of the alert type for flag comparison
    pub fn flag_id(&self) -> usize {
        self.alert_type.id()
    }
}

/// Custom implementation of display which excludes the timestamp. Used primarily for UUID
/// generation.
impl Display for NewNodeAlert {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?} {} {}", self.alert_type, self.machine_id, self.node_name)
    }
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct NodeActiveAlert {
    pub alert_id: Uuid,
    pub alert_type: Alert,
    pub machine_id: Uuid,
    pub organization_id: i64,
    pub client_id: Address,
    pub node_name: String,
    pub created_at: NaiveDateTime,
    pub acknowledged_at: Option<NaiveDateTime>,
    pub telegram_send: SendState,
    pub sendgrid_send: SendState,
    pub pagerduty_send: SendState,
}

pub struct DbNodeActiveAlert {
    alert_id: Uuid,
    machine_id: Uuid,
    organization_id: i64,
    client_id: Vec<u8>,
    node_name: String,
    created_at: NaiveDateTime,
    acknowledged_at: Option<NaiveDateTime>,
    alert_data: serde_json::Value,
    telegram_send: SendState,
    sendgrid_send: SendState,
    pagerduty_send: SendState,
}

impl From<DbNodeActiveAlert> for NodeActiveAlert {
    fn from(db_active_alert: DbNodeActiveAlert) -> Self {
        let notification_type: Alert = serde_json::from_value(db_active_alert.alert_data).unwrap();
        NodeActiveAlert {
            alert_id: db_active_alert.alert_id,
            alert_type: notification_type,
            machine_id: db_active_alert.machine_id,
            organization_id: db_active_alert.organization_id,
            client_id: Address::from_slice(&db_active_alert.client_id),
            node_name: db_active_alert.node_name,
            created_at: db_active_alert.created_at,
            acknowledged_at: db_active_alert.acknowledged_at,
            telegram_send: db_active_alert.telegram_send,
            sendgrid_send: db_active_alert.sendgrid_send,
            pagerduty_send: db_active_alert.pagerduty_send,
        }
    }
}

impl NodeActiveAlert {
    pub async fn get(
        pool: &PgPool,
        alert_id: Uuid,
    ) -> Result<Option<NodeActiveAlert>, DatabaseError> {
        let alert = sqlx::query_as!(
            DbNodeActiveAlert,
            r#"
            SELECT
                alert_id,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                alert_data,
                telegram_send AS "telegram_send!: SendState",
                sendgrid_send AS "sendgrid_send!: SendState",
                pagerduty_send AS "pagerduty_send!: SendState"
            FROM node_alerts_active
            WHERE alert_id = $1
            "#,
            alert_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(alert.map(|n| n.into()))
    }

    pub async fn get_by_avs_list(
        pool: &PgPool,
        nodes: &[Avs],
    ) -> Result<Vec<NodeActiveAlert>, DatabaseError> {
        // return alerts where alert machine_id and node_name match the corresponding
        // values from (avs.machine_id, avs.avs_name)
        let mut alerts = Vec::new();
        for node in nodes {
            let node_alerts = sqlx::query_as!(
                DbNodeActiveAlert,
                r#"
                SELECT
                    alert_id,
                    machine_id,
                    organization_id,
                    client_id,
                    node_name,
                    created_at,
                    acknowledged_at,
                    alert_data,
                    telegram_send AS "telegram_send!: SendState",
                    sendgrid_send AS "sendgrid_send!: SendState",
                    pagerduty_send AS "pagerduty_send!: SendState"
                FROM node_alerts_active
                WHERE machine_id = $1 AND node_name = $2
                "#,
                node.machine_id,
                node.avs_name
            )
            .fetch_all(pool)
            .await?;

            alerts.extend(node_alerts.into_iter().map(|n| n.into()));
        }
        Ok(alerts)
    }

    pub async fn get_many(
        pool: &PgPool,
        alert_ids: &[Uuid],
    ) -> Result<Vec<NodeActiveAlert>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbNodeActiveAlert,
            r#"
            SELECT
                alert_id,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                alert_data,
                telegram_send AS "telegram_send!: SendState",
                sendgrid_send AS "sendgrid_send!: SendState",
                pagerduty_send AS "pagerduty_send!: SendState"

            FROM node_alerts_active
            WHERE alert_id = ANY($1)
            "#,
            alert_ids
        )
        .fetch_all(pool)
        .await?;

        Ok(alerts.into_iter().map(|n| n.into()).collect())
    }

    pub async fn get_all(pool: &PgPool) -> Result<Vec<NodeActiveAlert>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbNodeActiveAlert,
            r#"
            SELECT
                alert_id,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                alert_data,
                telegram_send AS "telegram_send!: SendState",
                sendgrid_send AS "sendgrid_send!: SendState",
                pagerduty_send AS "pagerduty_send!: SendState"

            FROM node_alerts_active
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(alerts.into_iter().map(|n| n.into()).collect())
    }

    pub async fn insert_one(pool: &PgPool, alert: &NewNodeAlert) -> Result<(), DatabaseError> {
        let alert_data = serde_json::json!(alert.alert_type);
        println!("Inserting alert: {:#?}", alert);
        sqlx::query!(
            r#"
            INSERT INTO node_alerts_active (
                alert_id,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                alert_data,
                telegram_send,
                sendgrid_send,
                pagerduty_send
            )
            SELECT
                $1,
                m.machine_id,
                c.organization_id,
                m.client_id,
                $2,
                $3,
                $5,
                $6,
                $7,
                $8
            FROM machine m
            JOIN client c
              ON m.client_id = c.client_id
            WHERE m.machine_id = $4   -- lookup based on the provided machine_id
            "#,
            alert.id,
            alert.node_name,
            alert.created_at,
            alert.machine_id,
            alert_data,
            alert.telegram_send as SendState,
            alert.sendgrid_send as SendState,
            alert.pagerduty_send as SendState,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn insert_many(pool: &PgPool, alerts: &[NewNodeAlert]) -> Result<(), DatabaseError> {
        let mut tx = pool.begin().await?;
        for alert in alerts {
            let alert_data = serde_json::json!(alert.alert_type);
            sqlx::query!(
                r#"
            INSERT INTO node_alerts_active (
                alert_id,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                alert_data,
                telegram_send,
                sendgrid_send,
                pagerduty_send
            )
            SELECT
                $1,
                m.machine_id,
                c.organization_id,
                m.client_id,
                $2,
                $3,
                $5,
                $6,
                $7,
                $8
            FROM machine m
            JOIN client c
              ON m.client_id = c.client_id
            WHERE m.machine_id = $4
            "#,
                alert.id,
                alert.node_name,
                alert.created_at,
                alert.machine_id,
                alert_data,
                alert.telegram_send as SendState,
                alert.sendgrid_send as SendState,
                alert.pagerduty_send as SendState,
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
    ) -> Result<Vec<NodeActiveAlert>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbNodeActiveAlert,
            r#"
            SELECT
                alert_id,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                alert_data,
                telegram_send AS "telegram_send!: SendState",
                sendgrid_send AS "sendgrid_send!: SendState",
                pagerduty_send AS "pagerduty_send!: SendState"
            FROM node_alerts_active
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
    ) -> Result<Vec<NodeActiveAlert>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbNodeActiveAlert,
            r#"
            SELECT
                alert_id,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                alert_data,
                telegram_send AS "telegram_send!: SendState",
                sendgrid_send AS "sendgrid_send!: SendState",
                pagerduty_send AS "pagerduty_send!: SendState"
            FROM node_alerts_active
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
            UPDATE node_alerts_active
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
        let active_alert: NodeActiveAlert = sqlx::query_as!(
            DbNodeActiveAlert,
            r#"
            SELECT
                alert_id,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                alert_data,
                telegram_send AS "telegram_send!: SendState",
                sendgrid_send AS "sendgrid_send!: SendState",
                pagerduty_send AS "pagerduty_send!: SendState"
            FROM node_alerts_active
            WHERE alert_id = $1
            "#,
            alert_id
        )
        .fetch_one(pool) // Use the transaction if you want both queries to be part of the same transaction
        .await?
        .into();

        let alert_id = active_alert.alert_id;

        let history_alert: NodeHistoryAlert = active_alert.into();
        let alert_data = serde_json::json!(history_alert.alert_type);

        sqlx::query!(
            r#"
            INSERT INTO node_alerts_historical (
                alert_id,
                machine_id,
                organization_id,
                client_id,
                node_name,
                created_at,
                acknowledged_at,
                resolved_at,
                alert_data
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                now(),
                $8
            )
            "#,
            alert_id,
            history_alert.machine_id,
            history_alert.organization_id,
            history_alert.client_id.as_bytes().to_vec(),
            history_alert.node_name,
            history_alert.created_at,
            history_alert.acknowledged_at,
            alert_data
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            DELETE FROM node_alerts_active
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
