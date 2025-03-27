use chrono::{Local, NaiveDateTime};
use ivynet_alerts::Alert;
use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;

use super::alerts_active::OrganizationActiveAlert;

#[derive(Serialize, ToSchema, Clone, Debug)]
pub struct OrganizationHistoryAlert {
    pub alert_db_id: i64,
    pub alert_id: Uuid,
    pub alert_type: Alert,
    pub organization_id: i64,
    pub created_at: NaiveDateTime,
    pub acknowledged_at: NaiveDateTime,
}

pub struct DbOrganizationHistoryAlert {
    alert_db_id: i64,
    alert_id: Uuid,
    organization_id: i64,
    created_at: NaiveDateTime,
    acknowledged_at: NaiveDateTime,
    alert_data: serde_json::Value,
}

impl From<OrganizationHistoryAlert> for DbOrganizationHistoryAlert {
    fn from(value: OrganizationHistoryAlert) -> Self {
        Self {
            alert_db_id: value.alert_db_id,
            alert_id: value.alert_id,
            organization_id: value.organization_id,
            created_at: value.created_at,
            acknowledged_at: value.acknowledged_at,
            alert_data: serde_json::json!(value.alert_type),
        }
    }
}

impl From<DbOrganizationHistoryAlert> for OrganizationHistoryAlert {
    fn from(value: DbOrganizationHistoryAlert) -> Self {
        Self {
            alert_db_id: value.alert_db_id,
            alert_id: value.alert_id,
            alert_type: serde_json::from_value(value.alert_data)
                .expect("Could not deserialize alert type"),
            organization_id: value.organization_id,
            created_at: value.created_at,
            acknowledged_at: value.acknowledged_at,
        }
    }
}

impl From<OrganizationActiveAlert> for OrganizationHistoryAlert {
    fn from(value: OrganizationActiveAlert) -> Self {
        let now = Local::now().naive_utc();
        Self {
            alert_db_id: 0, // This will be set by the database
            alert_id: value.alert_id,
            alert_type: value.alert_type,
            organization_id: value.organization_id,
            created_at: value.created_at,
            acknowledged_at: now,
        }
    }
}

impl OrganizationHistoryAlert {
    pub async fn get(
        pool: &PgPool,
        alert_id: Uuid,
    ) -> Result<Option<OrganizationHistoryAlert>, DatabaseError> {
        let db_history_alert = sqlx::query_as!(
            DbOrganizationHistoryAlert,
            r#"
            SELECT
                alert_db_id,
                alert_id,
                organization_id,
                created_at,
                acknowledged_at,
                alert_data
            FROM
                organization_alerts_historical
            WHERE
                alert_id = $1
            "#,
            alert_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(db_history_alert.map(|a| a.into()))
    }

    pub async fn get_all(pool: &PgPool) -> Result<Vec<OrganizationHistoryAlert>, DatabaseError> {
        let db_history_alerts = sqlx::query_as!(
            DbOrganizationHistoryAlert,
            r#"
            SELECT
                alert_db_id,
                alert_id,
                organization_id,
                created_at,
                acknowledged_at,
                alert_data
            FROM
                organization_alerts_historical
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(db_history_alerts.into_iter().map(|a| a.into()).collect())
    }

    pub async fn record_new(
        pool: &PgPool,
        alert: &OrganizationHistoryAlert,
    ) -> Result<(), DatabaseError> {
        let alert_data = serde_json::json!(alert.alert_type);
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
            alert.alert_id,
            alert.organization_id,
            alert.created_at,
            alert.acknowledged_at,
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
    ) -> Result<Vec<OrganizationHistoryAlert>, DatabaseError> {
        let db_history_alerts = sqlx::query_as!(
            DbOrganizationHistoryAlert,
            r#"
            SELECT
                alert_db_id,
                alert_id,
                organization_id,
                created_at,
                acknowledged_at,
                alert_data
            FROM
                organization_alerts_historical
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
    ) -> Result<Vec<OrganizationHistoryAlert>, DatabaseError> {
        let db_history_alerts = sqlx::query_as!(
            DbOrganizationHistoryAlert,
            r#"
            SELECT
                alert_db_id,
                alert_id,
                organization_id,
                created_at,
                acknowledged_at,
                alert_data
            FROM
                organization_alerts_historical
            WHERE
                organization_id = $1
            "#,
            organization_id
        )
        .fetch_all(pool)
        .await?;

        Ok(db_history_alerts.into_iter().map(|a| a.into()).collect())
    }
}
