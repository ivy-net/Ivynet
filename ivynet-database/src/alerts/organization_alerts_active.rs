use std::fmt::{self, Display, Formatter};

use chrono::NaiveDateTime;
use ivynet_alerts::{Alert, Channel, SendState};
use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;

use super::organization_alerts_historical::OrganizationHistoryAlert;

#[derive(Debug, Clone, Serialize)]
pub struct NewOrganizationAlert {
    pub id: Uuid,
    pub alert_type: Alert,
    pub organization_id: i64,
    pub created_at: NaiveDateTime,
    pub telegram_send: SendState,
    pub sendgrid_send: SendState,
    pub pagerduty_send: SendState,
}

impl NewOrganizationAlert {
    pub fn new(organization_id: i64, alert_type: Alert) -> Self {
        let alert_id = alert_type.uuid_seed();
        let str_rep = format!("{}-{}", alert_id, organization_id);
        let id = Uuid::new_v5(&Uuid::NAMESPACE_OID, str_rep.as_bytes());
        Self {
            id,
            alert_type,
            organization_id,
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
impl Display for NewOrganizationAlert {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?} {}", self.alert_type, self.organization_id)
    }
}

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct OrganizationActiveAlert {
    pub alert_id: Uuid,
    pub alert_type: Alert,
    pub organization_id: i64,
    pub created_at: NaiveDateTime,
    pub telegram_send: SendState,
    pub sendgrid_send: SendState,
    pub pagerduty_send: SendState,
}

pub struct DbOrganizationActiveAlert {
    alert_id: Uuid,
    organization_id: i64,
    created_at: NaiveDateTime,
    alert_data: serde_json::Value,
    telegram_send: SendState,
    sendgrid_send: SendState,
    pagerduty_send: SendState,
}

impl From<DbOrganizationActiveAlert> for OrganizationActiveAlert {
    fn from(db_active_alert: DbOrganizationActiveAlert) -> Self {
        let notification_type: Alert = serde_json::from_value(db_active_alert.alert_data).unwrap();
        OrganizationActiveAlert {
            alert_id: db_active_alert.alert_id,
            alert_type: notification_type,
            organization_id: db_active_alert.organization_id,
            created_at: db_active_alert.created_at,
            telegram_send: db_active_alert.telegram_send,
            sendgrid_send: db_active_alert.sendgrid_send,
            pagerduty_send: db_active_alert.pagerduty_send,
        }
    }
}

impl OrganizationActiveAlert {
    pub async fn get(
        pool: &PgPool,
        alert_id: Uuid,
        organization_id: i64,
    ) -> Result<Option<OrganizationActiveAlert>, DatabaseError> {
        let alert = sqlx::query_as!(
            DbOrganizationActiveAlert,
            r#"
            SELECT
                alert_id,
                organization_id,
                created_at,
                alert_data,
                telegram_send AS "telegram_send!: SendState",
                sendgrid_send AS "sendgrid_send!: SendState",
                pagerduty_send AS "pagerduty_send!: SendState"
            FROM organization_alerts_active
            WHERE alert_id = $1 AND organization_id = $2
            "#,
            alert_id,
            organization_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(alert.map(|n| n.into()))
    }

    pub async fn get_many(
        pool: &PgPool,
        alert_ids: &[Uuid],
        organization_id: i64,
    ) -> Result<Vec<OrganizationActiveAlert>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbOrganizationActiveAlert,
            r#"
            SELECT
                alert_id,
                organization_id,
                created_at,
                alert_data,
                telegram_send AS "telegram_send!: SendState",
                sendgrid_send AS "sendgrid_send!: SendState",
                pagerduty_send AS "pagerduty_send!: SendState"
            FROM organization_alerts_active
            WHERE alert_id = ANY($1) AND organization_id = $2
            "#,
            alert_ids,
            organization_id
        )
        .fetch_all(pool)
        .await?;

        Ok(alerts.into_iter().map(|n| n.into()).collect())
    }

    pub async fn get_all(pool: &PgPool) -> Result<Vec<OrganizationActiveAlert>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbOrganizationActiveAlert,
            r#"
            SELECT
                alert_id,
                organization_id,
                created_at,
                alert_data,
                telegram_send AS "telegram_send!: SendState",
                sendgrid_send AS "sendgrid_send!: SendState",
                pagerduty_send AS "pagerduty_send!: SendState"
            FROM organization_alerts_active
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(alerts.into_iter().map(|n| n.into()).collect())
    }

    pub async fn insert_one(
        pool: &PgPool,
        alert: &NewOrganizationAlert,
    ) -> Result<(), DatabaseError> {
        let alert_data = serde_json::json!(alert.alert_type);
        sqlx::query!(
            r#"
            INSERT INTO organization_alerts_active (
                alert_id,
                organization_id,
                created_at,
                alert_data,
                telegram_send,
                sendgrid_send,
                pagerduty_send
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            alert.id,
            alert.organization_id,
            alert.created_at,
            alert_data,
            alert.telegram_send as SendState,
            alert.sendgrid_send as SendState,
            alert.pagerduty_send as SendState
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn insert_many(
        pool: &PgPool,
        alerts: &[NewOrganizationAlert],
    ) -> Result<(), DatabaseError> {
        let mut tx = pool.begin().await?;
        for alert in alerts {
            let alert_data = serde_json::json!(alert.alert_type);
            sqlx::query!(
                r#"
                INSERT INTO organization_alerts_active (
                    alert_id,
                    organization_id,
                    created_at,
                    alert_data,
                    telegram_send,
                    sendgrid_send,
                    pagerduty_send
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                "#,
                alert.id,
                alert.organization_id,
                alert.created_at,
                alert_data,
                alert.telegram_send as SendState,
                alert.sendgrid_send as SendState,
                alert.pagerduty_send as SendState
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
    ) -> Result<Vec<OrganizationActiveAlert>, DatabaseError> {
        let alerts = sqlx::query_as!(
            DbOrganizationActiveAlert,
            r#"
            SELECT
                alert_id,
                organization_id,
                created_at,
                alert_data,
                telegram_send AS "telegram_send!: SendState",
                sendgrid_send AS "sendgrid_send!: SendState",
                pagerduty_send AS "pagerduty_send!: SendState"
            FROM organization_alerts_active
            WHERE organization_id = $1
            "#,
            organization_id
        )
        .fetch_all(pool)
        .await?;

        Ok(alerts.into_iter().map(|n| n.into()).collect())
    }

    pub async fn resolve_alert(
        pool: &PgPool,
        alert_id: Uuid,
        organization_id: i64,
    ) -> Result<(), DatabaseError> {
        let mut tx = pool.begin().await?;

        // Get the active alert
        let active_alert: OrganizationActiveAlert = sqlx::query_as!(
            DbOrganizationActiveAlert,
            r#"
            SELECT
                alert_id,
                organization_id,
                created_at,
                alert_data,
                telegram_send AS "telegram_send!: SendState",
                sendgrid_send AS "sendgrid_send!: SendState",
                pagerduty_send AS "pagerduty_send!: SendState"
            FROM organization_alerts_active
            WHERE alert_id = $1 AND organization_id = $2
            "#,
            alert_id,
            organization_id
        )
        .fetch_one(&mut *tx)
        .await?
        .into();

        // Convert to historical alert
        let history_alert: OrganizationHistoryAlert = active_alert.into();
        let alert_data = serde_json::json!(history_alert.alert_type);

        // Insert into historical table
        sqlx::query!(
            r#"
            INSERT INTO organization_alerts_historical (
                alert_id,
                organization_id,
                created_at,
                acknowledged_at,
                alert_data
            )
            VALUES ($1, $2, $3, $4, $5)
            "#,
            alert_id,
            organization_id,
            history_alert.created_at,
            history_alert.acknowledged_at,
            alert_data
        )
        .execute(&mut *tx)
        .await?;

        // Delete from active table
        sqlx::query!(
            r#"
            DELETE FROM organization_alerts_active
            WHERE alert_id = $1 AND organization_id = $2
            "#,
            alert_id,
            organization_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}
