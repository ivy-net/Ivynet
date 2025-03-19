use std::collections::HashSet;

use chrono::NaiveDateTime;
use ivynet_alerts::AlertFlags;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::error::DatabaseError;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, sqlx::Type, Deserialize, Serialize, ToSchema)]
#[sqlx(type_name = "service_type", rename_all = "lowercase")]
pub enum ServiceType {
    Email,
    Telegram,
    PagerDuty,
}

#[derive(sqlx::FromRow, Deserialize, Serialize, Clone, Debug, Default)]
pub struct NotificationSettings {
    pub organization_id: i64,
    pub email: bool,
    pub telegram: bool,
    pub pagerduty: bool,
    pub alert_flags: AlertFlags,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(sqlx::FromRow, Deserialize, Serialize, Clone, Debug)]
pub struct ServiceSettings {
    pub organization_id: i64,
    pub settings_type: ServiceType,
    pub settings_value: String,
    pub created_at: Option<NaiveDateTime>,
}

impl NotificationSettings {
    pub async fn get(pool: &PgPool, id: u64) -> Result<Self, DatabaseError> {
        Ok(sqlx::query_as!(
            NotificationSettings,
            r#"SELECT
                    organization_id,
                    email, telegram, pagerduty, alert_flags,
                    created_at, updated_at
               FROM
                    notification_settings
               WHERE
                    organization_id = $1"#,
            id as i64
        )
        .fetch_one(pool)
        .await?)
    }

    pub async fn set(
        pool: &PgPool,
        id: u64,
        email: bool,
        telegram: bool,
        pagerduty: bool,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            r#"INSERT INTO
                notification_settings
                (organization_id, email, telegram, pagerduty, created_at, updated_at)
            VALUES
                ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (organization_id)
            DO UPDATE SET
                email = EXCLUDED.email, telegram = EXCLUDED.telegram,
                pagerduty = EXCLUDED.pagerduty, updated_at = EXCLUDED.updated_at"#,
            id as i64,
            email,
            telegram,
            pagerduty
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn add_chat(pool: &PgPool, id: u64, chat_id: &str) -> Result<(), DatabaseError> {
        sqlx::query_as!(
            ServiceSettings,
            r#"INSERT INTO
                service_settings
                (organization_id, settings_type, settings_value, created_at)
            VALUES
                ($1, $2, $3,  NOW())"#,
            id as i64,
            ServiceType::Telegram as ServiceType,
            chat_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Adds multiple Telegram chat IDs to the service settings in a single database operation.
    pub async fn add_many_chats(
        pool: &PgPool,
        id: u64,
        chats: &[String],
    ) -> Result<(), DatabaseError> {
        if chats.is_empty() {
            return Ok(());
        }

        // Deduplicate the chats using HashSet
        let chats: Vec<_> = chats.iter().collect::<HashSet<_>>().into_iter().collect();

        // Create a vector of tuples containing all the values for each row
        let values: Vec<(i64, ServiceType, String)> =
            chats.iter().map(|chat| (id as i64, ServiceType::Telegram, (*chat).clone())).collect();

        // Use SQLx's built-in support for bulk inserts
        sqlx::query!(
            r#"INSERT INTO service_settings
                (organization_id, settings_type, settings_value, created_at)
            SELECT org_id, type, value, NOW()
            FROM UNNEST($1::bigint[], $2::service_type[], $3::text[])
            AS t(org_id, type, value)"#,
            values.iter().map(|v| v.0).collect::<Vec<_>>() as Vec<i64>,
            values.iter().map(|v| v.1.clone()).collect::<Vec<_>>() as Vec<ServiceType>,
            values.iter().map(|v| v.2.clone()).collect::<Vec<_>>() as Vec<String>
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn chat_exists(pool: &PgPool, chat_id: &str) -> Result<bool, DatabaseError> {
        let result = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM service_settings WHERE settings_type = $1 AND settings_value = $2"#,
            ServiceType::Telegram as ServiceType,
            chat_id
        )
        .fetch_one(pool)
        .await?;

        Ok(result.unwrap_or(0) > 0)
    }

    pub async fn remove_chat(pool: &PgPool, chat_id: &str) -> Result<u64, DatabaseError> {
        let result = sqlx::query!(
            r#"DELETE FROM
                service_settings
               WHERE
                settings_type = $1 AND settings_value = $2"#,
            ServiceType::Telegram as ServiceType,
            chat_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn get_service_settings(
        pool: &PgPool,
        id: u64,
        settings_type: Option<ServiceType>,
    ) -> Result<Vec<ServiceSettings>, DatabaseError> {
        if let Some(settings_type) = settings_type {
            Ok(sqlx::query_as!(
                ServiceSettings,
                r#"SELECT
                organization_id, settings_type as "settings_type: _", settings_value, created_at
               FROM
                service_settings
               WHERE
                organization_id = $1 AND settings_type = $2"#,
                id as i64,
                settings_type as ServiceType,
            )
            .fetch_all(pool)
            .await?)
        } else {
            Ok(sqlx::query_as!(
                ServiceSettings,
                r#"SELECT
                organization_id, settings_type as "settings_type: _", settings_value, created_at
               FROM
                service_settings
               WHERE
                organization_id = $1"#,
                id as i64,
            )
            .fetch_all(pool)
            .await?)
        }
    }

    pub async fn get_all_chats(pool: &PgPool, id: u64) -> Result<Vec<String>, DatabaseError> {
        Ok(Self::get_service_settings(pool, id, Some(ServiceType::Telegram))
            .await?
            .iter()
            .map(|s| s.settings_value.clone())
            .collect::<Vec<_>>())
    }

    pub async fn get_all_emails(pool: &PgPool, id: u64) -> Result<Vec<String>, DatabaseError> {
        Ok(Self::get_service_settings(pool, id, Some(ServiceType::Email))
            .await?
            .iter()
            .map(|s| s.settings_value.clone())
            .collect::<Vec<_>>())
    }

    pub async fn get_pagerduty_integration(
        pool: &PgPool,
        id: u64,
    ) -> Result<Option<String>, DatabaseError> {
        Ok(Self::get_service_settings(pool, id, Some(ServiceType::PagerDuty))
            .await?
            .iter()
            .map(|s| s.settings_value.clone())
            .next())
    }

    pub async fn set_emails(
        pool: &PgPool,
        id: u64,
        emails: &[String],
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            r#"DELETE FROM
                service_settings
               WHERE
                organization_id = $1 AND settings_type = $2"#,
            id as i64,
            ServiceType::Email as ServiceType,
        )
        .execute(pool)
        .await?;

        for email in emails {
            sqlx::query!(
                r#"INSERT INTO
                service_settings
                (organization_id, settings_type, settings_value, created_at)
               VALUES
                ($1, $2, $3, NOW())"#,
                id as i64,
                ServiceType::Email as ServiceType,
                email
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    pub async fn set_pagerduty_integration(
        pool: &PgPool,
        id: u64,
        integration_id: &str,
    ) -> Result<(), DatabaseError> {
        sqlx::query_as!(
            ServiceSettings,
            r#"INSERT INTO
                service_settings
                (organization_id, settings_type, settings_value, created_at)
            VALUES
                ($1, $2, $3,  NOW())"#,
            id as i64,
            ServiceType::PagerDuty as ServiceType,
            integration_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn set_alert_flags(pool: &PgPool, id: u64, flags: u64) -> Result<(), DatabaseError> {
        sqlx::query!(
            r#"UPDATE
                notification_settings
               SET
                alert_flags = $2
               WHERE
                organization_id = $1"#,
            id as i64,
            flags as i64
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_alert_flags(pool: &PgPool, id: u64) -> Result<u64, DatabaseError> {
        Ok(sqlx::query!(
            r#"SELECT
                alert_flags
               FROM
                notification_settings
               WHERE
                organization_id = $1"#,
            id as i64
        )
        .fetch_one(pool)
        .await?
        .alert_flags as u64)
    }
}
