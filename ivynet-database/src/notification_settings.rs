use std::collections::HashSet;

use chrono::NaiveDateTime;
use ivynet_alerts::AlertFlags;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, Default)]
pub struct NotificationSettings {
    pub organization_id: i64,
    pub email: bool,
    pub telegram: bool,
    pub pagerduty: bool,
    pub alert_flags: AlertFlags,
    pub sendgrid_emails: HashSet<String>,
    pub telegram_chats: HashSet<String>,
    pub pagerduty_keys: HashSet<String>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl From<NotificationSettingsRow> for NotificationSettings {
    fn from(row: NotificationSettingsRow) -> Self {
        NotificationSettings {
            organization_id: row.organization_id,
            alert_flags: row.alert_flags,
            email: row.email,
            telegram: row.telegram,
            pagerduty: row.pagerduty,
            sendgrid_emails: row.sendgrid_emails.into_iter().collect(),
            telegram_chats: row.telegram_chats.into_iter().collect(),
            pagerduty_keys: row.pagerduty_keys.into_iter().collect(),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct NotificationSettingsRow {
    organization_id: i64,
    email: bool,
    telegram: bool,
    pagerduty: bool,
    alert_flags: AlertFlags,
    created_at: Option<NaiveDateTime>,
    updated_at: Option<NaiveDateTime>,
    sendgrid_emails: Vec<String>,
    telegram_chats: Vec<String>,
    pagerduty_keys: Vec<String>,
}

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, sqlx::Type, Deserialize, Serialize, ToSchema,
)]
#[sqlx(type_name = "service_type", rename_all = "lowercase")]
pub enum ServiceType {
    Email,
    Telegram,
    PagerDuty,
}

#[derive(sqlx::FromRow, Deserialize, Serialize, Clone, Debug)]
pub struct ServiceSettings {
    pub organization_id: i64,
    pub settings_type: ServiceType,
    pub settings_value: String,
    pub created_at: Option<NaiveDateTime>,
}

impl ServiceSettings {
    fn uuid(&self) -> Uuid {
        let uuid_seed =
            format!("{}-{:?}-{}", self.organization_id, self.settings_type, self.settings_value);
        Uuid::new_v5(&Uuid::NAMESPACE_URL, &uuid_seed.as_bytes())
    }

    // Create or update a service setting. Returns the UUID of the setting if it was created or
    // None if it already existed.
    pub async fn create(
        pool: &PgPool,
        org_id: u64,
        settings_type: ServiceType,
        value: &str,
    ) -> Result<Option<Uuid>, DatabaseError> {
        let service_setting = Self {
            organization_id: org_id as i64,
            settings_type,
            settings_value: value.to_string(),
            created_at: None,
        };

        let uuid = service_setting.uuid();

        sqlx::query!(
            r#"INSERT INTO
                service_settings
                (id, organization_id, settings_type, settings_value, created_at)
            VALUES
                ($1, $2, $3, $4, NOW())
            ON CONFLICT (id)
            DO NOTHING"#,
            uuid,
            org_id as i64,
            settings_type as ServiceType,
            value
        )
        .execute(pool)
        .await?;

        Ok(Some(uuid))
    }

    // Delete a service setting by UUID
    pub async fn delete_by_uuid(pool: &PgPool, uuid: Uuid) -> Result<u64, DatabaseError> {
        let result = sqlx::query!(
            r#"DELETE FROM
                service_settings
               WHERE
                id = $1"#,
            uuid
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    // Delete service settings by organization ID and type
    pub async fn delete_by_org_and_type(
        pool: &PgPool,
        org_id: u64,
        settings_type: ServiceType,
    ) -> Result<u64, DatabaseError> {
        let result = sqlx::query!(
            r#"DELETE FROM
                service_settings
               WHERE
                organization_id = $1 AND settings_type = $2"#,
            org_id as i64,
            settings_type as ServiceType
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn remove_chat(pool: &PgPool, chat_id: &str) -> Result<u64, DatabaseError> {
        let result = sqlx::query!(
            r#"DELETE FROM
                service_settings
               WHERE
                settings_type = 'telegram' AND settings_value = $1"#,
            chat_id
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    // Get all service settings for an organization with optional type filter
    pub async fn get_for_org(
        pool: &PgPool,
        org_id: u64,
        settings_type: Option<ServiceType>,
    ) -> Result<Vec<Self>, DatabaseError> {
        if let Some(settings_type) = settings_type {
            Ok(sqlx::query_as!(
                Self,
                r#"SELECT
                    organization_id, settings_type as "settings_type: _", settings_value, created_at
                FROM
                    service_settings
                WHERE
                    organization_id = $1 AND settings_type = $2"#,
                org_id as i64,
                settings_type as ServiceType,
            )
            .fetch_all(pool)
            .await?)
        } else {
            Ok(sqlx::query_as!(
                Self,
                r#"SELECT
                    organization_id, settings_type as "settings_type: _", settings_value, created_at
                FROM
                    service_settings
                WHERE
                    organization_id = $1"#,
                org_id as i64,
            )
            .fetch_all(pool)
            .await?)
        }
    }
}

impl NotificationSettings {
    pub async fn get(pool: &PgPool, id: u64) -> Result<Self, DatabaseError> {
        // First, get the base notification settings with all service settings
        let row = sqlx::query_as!(
            NotificationSettingsRow,
            r#"
            SELECT
                ns.organization_id,
                ns.email,
                ns.telegram,
                ns.pagerduty,
                ns.alert_flags,
                ns.created_at,
                ns.updated_at,
                ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'email' THEN ss.settings_value ELSE NULL END) FILTER (WHERE ss.settings_type = 'email') as "sendgrid_emails!: Vec<String>",
                ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'telegram' THEN ss.settings_value ELSE NULL END) FILTER (WHERE ss.settings_type = 'telegram') as "telegram_chats!: Vec<String>",
                ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'pagerduty' THEN ss.settings_value ELSE NULL END) FILTER (WHERE ss.settings_type = 'pagerduty') as "pagerduty_keys!: Vec<String>"
            FROM
                notification_settings ns
            LEFT JOIN
                service_settings ss ON ns.organization_id = ss.organization_id
            WHERE
                ns.organization_id = $1
            GROUP BY
                ns.organization_id, ns.email, ns.telegram, ns.pagerduty, ns.alert_flags, ns.created_at, ns.updated_at
            "#,
            id as i64
        )
        .fetch_one(pool)
        .await?;
        Ok(NotificationSettings::from(row))
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

    // TODO: Deprecate for above, more descriptive name
    pub async fn get_service_settings(
        pool: &PgPool,
        org_id: u64,
        settings_type: Option<ServiceType>,
    ) -> Result<Vec<ServiceSettings>, DatabaseError> {
        ServiceSettings::get_for_org(pool, org_id, settings_type).await
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

    pub async fn set_alert_flags(pool: &PgPool, id: u64, flags: u64) -> Result<(), DatabaseError> {
        sqlx::query!(
            r#"UPDATE
                notification_settings
               SET
                alert_flags = $2,
                updated_at = NOW()
               WHERE
                organization_id = $1"#,
            id as i64,
            flags as i64
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    // Add a single email
    pub async fn add_email(
        pool: &PgPool,
        id: u64,
        email: &str,
    ) -> Result<Option<Uuid>, DatabaseError> {
        ServiceSettings::create(pool, id, ServiceType::Email, email).await
    }

    // Add a single chat
    pub async fn add_chat(
        pool: &PgPool,
        id: u64,
        chat_id: &str,
    ) -> Result<Option<Uuid>, DatabaseError> {
        ServiceSettings::create(pool, id, ServiceType::Telegram, chat_id).await
    }

    // Add a pagerduty key
    pub async fn add_pagerduty_key(
        pool: &PgPool,
        id: u64,
        key: &str,
    ) -> Result<Option<Uuid>, DatabaseError> {
        ServiceSettings::create(pool, id, ServiceType::PagerDuty, key).await
    }

    // Methods for adding multiple items at once
    // TODO: use a transaction here
    pub async fn set_emails(
        pool: &PgPool,
        id: u64,
        emails: &[String],
    ) -> Result<Vec<Uuid>, DatabaseError> {
        if emails.is_empty() {
            return Ok(vec![]);
        }

        let mut uuids = Vec::with_capacity(emails.len());

        for email in emails {
            let uuid = Self::add_email(pool, id, email).await?;
            if let Some(uuid) = uuid {
                uuids.push(uuid);
            }
        }

        Ok(uuids)
    }

    pub async fn add_many_chats(
        pool: &PgPool,
        id: u64,
        chats: &[String],
    ) -> Result<Vec<Uuid>, DatabaseError> {
        if chats.is_empty() {
            return Ok(vec![]);
        }

        let mut uuids = Vec::with_capacity(chats.len());

        for chat in chats {
            let uuid = Self::add_chat(pool, id, chat).await?;
            if let Some(uuid) = uuid {
                uuids.push(uuid);
            }
        }

        Ok(uuids)
    }

    pub async fn add_pagerduty_keys(
        pool: &PgPool,
        id: u64,
        keys: &[String],
    ) -> Result<Vec<Uuid>, DatabaseError> {
        if keys.is_empty() {
            return Ok(vec![]);
        }

        let mut uuids = Vec::with_capacity(keys.len());

        for key in keys {
            let uuid = Self::add_pagerduty_key(pool, id, key).await?;
            if let Some(uuid) = uuid {
                uuids.push(uuid);
            }
        }

        Ok(uuids)
    }

    // TODO: Deprecate this in favor of the above.
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
    // Removing by UUID directly
    pub async fn remove_by_uuid(pool: &PgPool, uuid: Uuid) -> Result<u64, DatabaseError> {
        ServiceSettings::delete_by_uuid(pool, uuid).await
    }

    // Removing service settings by reconstructing their UUID
    pub async fn remove_email(pool: &PgPool, id: u64, email: &str) -> Result<u64, DatabaseError> {
        let service_setting = ServiceSettings {
            organization_id: id as i64,
            settings_type: ServiceType::Email,
            settings_value: email.to_string(),
            created_at: None,
        };

        Self::remove_by_uuid(pool, service_setting.uuid()).await
    }

    pub async fn remove_chat(pool: &PgPool, id: u64, chat_id: &str) -> Result<u64, DatabaseError> {
        let service_setting = ServiceSettings {
            organization_id: id as i64,
            settings_type: ServiceType::Telegram,
            settings_value: chat_id.to_string(),
            created_at: None,
        };

        Self::remove_by_uuid(pool, service_setting.uuid()).await
    }

    pub async fn remove_pagerduty_key(
        pool: &PgPool,
        id: u64,
        key: &str,
    ) -> Result<u64, DatabaseError> {
        let service_setting = ServiceSettings {
            organization_id: id as i64,
            settings_type: ServiceType::PagerDuty,
            settings_value: key.to_string(),
            created_at: None,
        };

        Self::remove_by_uuid(pool, service_setting.uuid()).await
    }
}
