use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::error::DatabaseError;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, sqlx::Type, Deserialize, Serialize, ToSchema)]
#[sqlx(type_name = "notification_type", rename_all = "lowercase")]
pub enum SettingsType {
    Email,
    Telegram,
    PagerDuty,
}

#[derive(sqlx::FromRow, Deserialize, Serialize, Clone, Debug)]
pub struct OrganizationNotifications {
    pub organization_id: i64,
    pub email: bool,
    pub telegram: bool,
    pub pagerduty: bool,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl Default for OrganizationNotifications {
    fn default() -> Self {
        Self {
            organization_id: 0,
            email: false,
            telegram: false,
            pagerduty: false,
            created_at: None,
            updated_at: None,
        }
    }
}

#[derive(sqlx::FromRow, Deserialize, Serialize, Clone, Debug)]
pub struct OrganizationNotificationsSettings {
    pub organization_id: i64,
    pub settings_type: SettingsType,
    pub settings_value: String,
    pub created_at: Option<NaiveDateTime>,
}

impl OrganizationNotifications {
    pub async fn get(pool: &PgPool, id: u64) -> Result<Self, DatabaseError> {
        Ok(sqlx::query_as!(
            OrganizationNotifications,
            r#"SELECT
                    organization_id,
                    email, telegram, pagerduty,
                    created_at, updated_at
               FROM
                    organization_notifications
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
                organization_notifications
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

    pub async fn add_chat(pool: &PgPool, id: u64, chat_id: String) -> Result<(), DatabaseError> {
        sqlx::query_as!(
            OrganizationNotificationsSettings,
            r#"INSERT INTO
                notification_settings
                (organization_id, settings_type, settings_value, created_at)
            VALUES
                ($1, $2, $3,  NOW())"#,
            id as i64,
            SettingsType::Telegram as SettingsType,
            chat_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn remove_chat(pool: &PgPool, id: u64, chat_id: String) -> Result<(), DatabaseError> {
        sqlx::query!(
            r#"DELETE FROM
                notification_settings
               WHERE
                organization_id = $1 AND settings_type = $2 AND settings_value = $3"#,
            id as i64,
            SettingsType::Telegram as SettingsType,
            chat_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_notification_settings(
        pool: &PgPool,
        id: u64,
        settings_type: Option<SettingsType>,
    ) -> Result<Vec<OrganizationNotificationsSettings>, DatabaseError> {
        if let Some(settings_type) = settings_type {
            Ok(sqlx::query_as!(
                OrganizationNotificationsSettings,
                r#"SELECT
                organization_id, settings_type as "settings_type: _", settings_value, created_at
               FROM
                notification_settings
               WHERE
                organization_id = $1 AND settings_type = $2"#,
                id as i64,
                settings_type as SettingsType,
            )
            .fetch_all(pool)
            .await?)
        } else {
            Ok(sqlx::query_as!(
                OrganizationNotificationsSettings,
                r#"SELECT
                organization_id, settings_type as "settings_type: _", settings_value, created_at
               FROM
                notification_settings
               WHERE
                organization_id = $1"#,
                id as i64,
            )
            .fetch_all(pool)
            .await?)
        }
    }

    pub async fn get_all_chats(pool: &PgPool, id: u64) -> Result<Vec<String>, DatabaseError> {
        Ok(Self::get_notification_settings(pool, id, Some(SettingsType::Telegram))
            .await?
            .iter()
            .map(|s| s.settings_value.clone())
            .collect::<Vec<_>>())
    }

    pub async fn get_all_emails(pool: &PgPool, id: u64) -> Result<Vec<String>, DatabaseError> {
        Ok(Self::get_notification_settings(pool, id, Some(SettingsType::Email))
            .await?
            .iter()
            .map(|s| s.settings_value.clone())
            .collect::<Vec<_>>())
    }

    pub async fn get_pagerduty_integration(
        pool: &PgPool,
        id: u64,
    ) -> Result<Option<String>, DatabaseError> {
        Ok(Self::get_notification_settings(pool, id, Some(SettingsType::PagerDuty))
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
                notification_settings
               WHERE
                organization_id = $1 AND settings_type = $2"#,
            id as i64,
            SettingsType::Email as SettingsType,
        )
        .execute(pool)
        .await?;

        for email in emails {
            sqlx::query!(
                r#"INSERT INTO
                notification_settings
                (organization_id, settings_type, settings_value, created_at)
               VALUES
                ($1, $2, $3, NOW())"#,
                id as i64,
                SettingsType::Email as SettingsType,
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
            OrganizationNotificationsSettings,
            r#"INSERT INTO
                notification_settings
                (organization_id, settings_type, settings_value, created_at)
            VALUES
                ($1, $2, $3,  NOW())"#,
            id as i64,
            SettingsType::PagerDuty as SettingsType,
            integration_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
