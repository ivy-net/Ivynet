use std::collections::HashSet;

use chrono::NaiveDateTime;
use ethers::types::Address;
use ivynet_alerts::AlertFlags;
use ivynet_notifications::Channel;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{error::DatabaseError, service_settings::ServiceType, ServiceSettings};

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

// Object-based methods
impl NotificationSettings {
    /// Get all active channels for the organization. Returns a Vec<Channel(targets)> where targets
    /// is a Vec<String> of the target addresses for the channel.
    pub fn get_active_channels(&self) -> Vec<Channel> {
        let mut channels = Vec::new();

        if self.email {
            channels.push(Channel::Email(self.sendgrid_emails.clone()));
        }

        if self.telegram {
            channels.push(Channel::Telegram(self.telegram_chats.clone()));
        }

        if self.pagerduty {
            channels.push(Channel::PagerDuty(self.pagerduty_keys.clone()));
        }

        channels
    }
}

// Databse access methods
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
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'email' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'email'), ARRAY[]::text[]) as "sendgrid_emails!: Vec<String>",
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'telegram' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'telegram'), ARRAY[]::text[]) as "telegram_chats!: Vec<String>",
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'pagerduty' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'pagerduty'), ARRAY[]::text[]) as "pagerduty_keys!: Vec<String>"
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

    pub async fn get_many(pool: &PgPool, ids: &[i64]) -> Result<Vec<Self>, DatabaseError> {
        let rows = sqlx::query_as!(
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
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'email' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'email'), ARRAY[]::text[]) as "sendgrid_emails!: Vec<String>",
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'telegram' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'telegram'), ARRAY[]::text[]) as "telegram_chats!: Vec<String>",
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'pagerduty' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'pagerduty'), ARRAY[]::text[]) as "pagerduty_keys!: Vec<String>"
            FROM
                notification_settings ns
            LEFT JOIN
                service_settings ss ON ns.organization_id = ss.organization_id
            WHERE
                ns.organization_id = ANY($1)
            GROUP BY
                ns.organization_id, ns.email, ns.telegram, ns.pagerduty, ns.alert_flags, ns.created_at, ns.updated_at
            "#,
            ids
        ).fetch_all(pool).await?;

        Ok(rows.into_iter().map(NotificationSettings::from).collect())
    }

    /// Fetch NotificationSettings for an organization corresponding to a given client ID.
    pub async fn get_for_client(pool: &PgPool, client_id: Address) -> Result<Self, DatabaseError> {
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
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'email' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'email'), ARRAY[]::text[]) as "sendgrid_emails!: Vec<String>",
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'telegram' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'telegram'), ARRAY[]::text[]) as "telegram_chats!: Vec<String>",
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'pagerduty' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'pagerduty'), ARRAY[]::text[]) as "pagerduty_keys!: Vec<String>"
            FROM
                notification_settings ns
            JOIN
                client c ON ns.organization_id = c.organization_id
            LEFT JOIN
                service_settings ss ON ns.organization_id = ss.organization_id
            WHERE
                c.client_id = $1
            GROUP BY
                ns.organization_id, ns.email, ns.telegram, ns.pagerduty, ns.alert_flags, ns.created_at, ns.updated_at
            "#,
            client_id.as_bytes() as &[u8]
        )
        .fetch_one(pool)
        .await?;
        Ok(NotificationSettings::from(row))
    }

    pub async fn get_for_machine(
        pool: &PgPool,
        machine_id: uuid::Uuid,
    ) -> Result<Self, DatabaseError> {
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
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'email' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'email'), ARRAY[]::text[]) as "sendgrid_emails!: Vec<String>",
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'telegram' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'telegram'), ARRAY[]::text[]) as "telegram_chats!: Vec<String>",
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'pagerduty' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'pagerduty'), ARRAY[]::text[]) as "pagerduty_keys!: Vec<String>"
            FROM
                notification_settings ns
            JOIN
                client c ON ns.organization_id = c.organization_id
            JOIN
                machine m ON c.client_id = m.client_id
            LEFT JOIN
                service_settings ss ON ns.organization_id = ss.organization_id
            WHERE
                m.machine_id = $1
            GROUP BY
                ns.organization_id, ns.email, ns.telegram, ns.pagerduty, ns.alert_flags, ns.created_at, ns.updated_at
            "#,
            machine_id
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
    pub async fn add_emails(
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

    /// Fetch NotificationSettings for organizations corresponding to multiple client IDs.
    pub async fn get_many_for_client(
        pool: &PgPool,
        client_ids: &[Address],
    ) -> Result<Vec<Self>, DatabaseError> {
        // Convert addresses to byte arrays for SQL
        let client_id_bytes: Vec<&[u8]> = client_ids.iter().map(|id| id.as_bytes()).collect();

        let rows = sqlx::query_as!(
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
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'email' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'email'), ARRAY[]::text[]) as "sendgrid_emails!: Vec<String>",
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'telegram' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'telegram'), ARRAY[]::text[]) as "telegram_chats!: Vec<String>",
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'pagerduty' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'pagerduty'), ARRAY[]::text[]) as "pagerduty_keys!: Vec<String>"
            FROM
                notification_settings ns
            JOIN
                client c ON ns.organization_id = c.organization_id
            LEFT JOIN
                service_settings ss ON ns.organization_id = ss.organization_id
            WHERE
                c.client_id = ANY($1)
            GROUP BY
                ns.organization_id, ns.email, ns.telegram, ns.pagerduty, ns.alert_flags, ns.created_at, ns.updated_at
            "#,
            &client_id_bytes as &[&[u8]]
        ).fetch_all(pool).await?;

        Ok(rows.into_iter().map(NotificationSettings::from).collect())
    }

    /// Fetch NotificationSettings for organizations corresponding to multiple machine IDs.
    pub async fn get_many_for_machine(
        pool: &PgPool,
        machine_ids: &[uuid::Uuid],
    ) -> Result<Vec<Self>, DatabaseError> {
        let rows = sqlx::query_as!(
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
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'email' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'email'), ARRAY[]::text[]) as "sendgrid_emails!: Vec<String>",
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'telegram' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'telegram'), ARRAY[]::text[]) as "telegram_chats!: Vec<String>",
                COALESCE(ARRAY_AGG(DISTINCT CASE WHEN ss.settings_type = 'pagerduty' THEN ss.settings_value END) FILTER (WHERE ss.settings_type = 'pagerduty'), ARRAY[]::text[]) as "pagerduty_keys!: Vec<String>"
            FROM
                notification_settings ns
            JOIN
                client c ON ns.organization_id = c.organization_id
            JOIN
                machine m ON c.client_id = m.client_id
            LEFT JOIN
                service_settings ss ON ns.organization_id = ss.organization_id
            WHERE
                m.machine_id = ANY($1)
            GROUP BY
                ns.organization_id, ns.email, ns.telegram, ns.pagerduty, ns.alert_flags, ns.created_at, ns.updated_at
            "#,
            machine_ids
        ).fetch_all(pool).await?;

        Ok(rows.into_iter().map(NotificationSettings::from).collect())
    }
}

#[cfg(test)]
mod notification_settings_tests {
    use super::*;
    use sqlx::PgPool;

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../fixtures/new_user_registration.sql", "../fixtures/notification_settings.sql")
    )]
    async fn test_get_notification_settings(pool: PgPool) {
        // Get settings for organization with ID 1
        let settings = NotificationSettings::get(&pool, 1).await.unwrap();

        // Verify the settings match what we set in the fixture
        assert_eq!(settings.organization_id, 1);
        assert!(settings.email);
        assert!(!settings.telegram);
        assert!(settings.pagerduty);
        assert_eq!(settings.alert_flags.as_u64(), 2_u64);

        // Check that we retrieved the service settings correctly
        assert_eq!(settings.sendgrid_emails.len(), 2);
        assert!(settings.sendgrid_emails.contains("test1@example.com"));
        assert!(settings.sendgrid_emails.contains("test2@example.com"));
        assert_eq!(settings.telegram_chats.len(), 0);
        assert_eq!(settings.pagerduty_keys.len(), 1);
        assert!(settings.pagerduty_keys.contains("pdkey123"));
    }

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../fixtures/new_user_registration.sql", "../fixtures/notification_settings.sql")
    )]
    async fn test_get_many_notification_settings(pool: PgPool) {
        let settings = NotificationSettings::get_many(&pool, &[1, 2]).await.unwrap();

        assert_eq!(settings.len(), 2);

        // Verify the settings exist
        assert!(settings.iter().any(|s| s.organization_id == 1));
        assert!(settings.iter().any(|s| s.organization_id == 2));
    }

    #[sqlx::test(migrations = "../migrations", fixtures("../fixtures/new_user_registration.sql"))]
    #[ignore]
    async fn test_set_notification_settings(pool: PgPool) {
        // Create new settings for organization with ID 1
        NotificationSettings::set(&pool, 1, true, true, false).await.unwrap();

        // Retrieve the settings we just created
        let settings = NotificationSettings::get(&pool, 1).await.unwrap();

        // Verify the settings match what we set
        assert_eq!(settings.organization_id, 1);
        assert!(settings.email);
        assert!(settings.telegram);
        assert!(!settings.pagerduty);

        // By default, no service settings should exist yet
        assert_eq!(settings.sendgrid_emails.len(), 0);
        assert_eq!(settings.telegram_chats.len(), 0);
        assert_eq!(settings.pagerduty_keys.len(), 0);

        // Now update the settings
        NotificationSettings::set(&pool, 1, false, true, true).await.unwrap();

        // Check the updated settings
        let updated_settings = NotificationSettings::get(&pool, 1).await.unwrap();
        assert!(!updated_settings.email);
        assert!(updated_settings.telegram);
        assert!(updated_settings.pagerduty);
    }

    #[ignore]
    #[sqlx::test(migrations = "../migrations", fixtures("../fixtures/new_user_registration.sql"))]
    async fn test_add_service_settings(pool: PgPool) {
        // First set up the notification settings
        NotificationSettings::set(&pool, 1, true, true, true).await.unwrap();

        // Add service settings
        let email1 = "user1@example.com";
        let email2 = "user2@example.com";
        let chat1 = "123456789";
        let key1 = "pdkey456";

        // Add individual settings
        let email_uuid = NotificationSettings::add_email(&pool, 1, email1).await.unwrap();
        let chat_uuid = NotificationSettings::add_chat(&pool, 1, chat1).await.unwrap();
        let pd_uuid = NotificationSettings::add_pagerduty_key(&pool, 1, key1).await.unwrap();

        // Verify UUIDs were returned
        assert!(email_uuid.is_some());
        assert!(chat_uuid.is_some());
        assert!(pd_uuid.is_some());

        // Add multiple emails
        let emails = vec![email1.to_string(), email2.to_string()];
        let email_uuids = NotificationSettings::add_emails(&pool, 1, &emails).await.unwrap();

        // We should only get one UUID back since email1 already exists
        assert_eq!(email_uuids.len(), 2);

        // Get the settings and verify
        let settings = NotificationSettings::get(&pool, 1).await.unwrap();

        // We should have both emails
        assert_eq!(settings.sendgrid_emails.len(), 2);
        assert!(settings.sendgrid_emails.contains(email1));
        assert!(settings.sendgrid_emails.contains(email2));

        // And the chat and pagerduty key
        assert_eq!(settings.telegram_chats.len(), 1);
        assert!(settings.telegram_chats.contains(chat1));
        assert_eq!(settings.pagerduty_keys.len(), 1);
        assert!(settings.pagerduty_keys.contains(key1));
    }

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../fixtures/new_user_registration.sql", "../fixtures/notification_settings.sql")
    )]
    async fn test_remove_service_settings(pool: PgPool) {
        // Get the initial settings
        let settings = NotificationSettings::get(&pool, 1).await.unwrap();
        assert_eq!(settings.sendgrid_emails.len(), 2);

        // Remove an email
        let removed =
            NotificationSettings::remove_email(&pool, 1, "test1@example.com").await.unwrap();
        assert_eq!(removed, 1); // 1 row affected

        // Verify it was removed
        let updated = NotificationSettings::get(&pool, 1).await.unwrap();
        assert_eq!(updated.sendgrid_emails.len(), 1);
        assert!(!updated.sendgrid_emails.contains("test1@example.com"));
        assert!(updated.sendgrid_emails.contains("test2@example.com"));

        // Add and then remove a chat
        let chat_id = "987654321";
        NotificationSettings::add_chat(&pool, 1, chat_id).await.unwrap();
        let removed = NotificationSettings::remove_chat(&pool, 1, chat_id).await.unwrap();
        assert_eq!(removed, 1);

        // Verify chat was removed
        let final_settings = NotificationSettings::get(&pool, 1).await.unwrap();
        assert!(!final_settings.telegram_chats.contains(chat_id));
    }

    #[ignore]
    #[sqlx::test(migrations = "../migrations", fixtures("../fixtures/new_user_registration.sql"))]
    async fn test_alert_flags(pool: PgPool) {
        // First set up notification settings
        NotificationSettings::set(&pool, 1, true, false, false).await.unwrap();

        // Set alert flags
        let flags: u64 = 42; // 101010 in binary
        NotificationSettings::set_alert_flags(&pool, 1, flags).await.unwrap();

        // Get and verify flags
        let retrieved_flags = NotificationSettings::get_alert_flags(&pool, 1).await.unwrap();
        assert_eq!(retrieved_flags, flags);

        // Verify flags via get as well
        let settings = NotificationSettings::get(&pool, 1).await.unwrap();
        assert_eq!(settings.alert_flags.as_u64(), flags);
    }

    #[ignore]
    #[sqlx::test(migrations = "../migrations", fixtures("../fixtures/new_user_registration.sql"))]
    async fn test_service_settings_methods(pool: PgPool) {
        // Test ServiceSettings::create and get_for_org methods

        // Create some service settings
        let org_id = 1u64;
        let uuid1 = ServiceSettings::create(&pool, org_id, ServiceType::Email, "new@example.com")
            .await
            .unwrap();
        let uuid2 =
            ServiceSettings::create(&pool, org_id, ServiceType::Telegram, "chat123").await.unwrap();

        assert!(uuid1.is_some());
        assert!(uuid2.is_some());

        // Get all settings for the org
        let all_settings = ServiceSettings::get_for_org(&pool, org_id, None).await.unwrap();
        assert_eq!(all_settings.len(), 2);

        // Get only email settings
        let email_settings =
            ServiceSettings::get_for_org(&pool, org_id, Some(ServiceType::Email)).await.unwrap();
        assert_eq!(email_settings.len(), 1);
        assert_eq!(email_settings[0].settings_value, "new@example.com");

        // Delete by UUID
        let deleted = ServiceSettings::delete_by_uuid(&pool, uuid1.unwrap()).await.unwrap();
        assert_eq!(deleted, 1);

        // Delete by org and type
        let deleted = ServiceSettings::delete_by_org_and_type(&pool, org_id, ServiceType::Telegram)
            .await
            .unwrap();
        assert_eq!(deleted, 1);

        // Verify all were deleted
        let remaining = ServiceSettings::get_for_org(&pool, org_id, None).await.unwrap();
        assert_eq!(remaining.len(), 0);
    }

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../fixtures/new_user_registration.sql", "../fixtures/notification_settings.sql")
    )]
    async fn test_get_notification_settings_for_client(pool: PgPool) {
        let client_id = "0x0101010101010101010101010101010101010101".parse::<Address>().unwrap();

        NotificationSettings::set(&pool, 1, true, false, true).await.unwrap();
        NotificationSettings::add_email(&pool, 1, "client_test@example.com").await.unwrap();

        let settings = NotificationSettings::get_for_client(&pool, client_id).await.unwrap();

        assert_eq!(settings.organization_id, 1);
        assert!(settings.email);
        assert!(!settings.telegram);
        assert!(settings.pagerduty);

        assert!(settings.sendgrid_emails.contains("client_test@example.com"));
    }

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../fixtures/new_user_registration.sql", "../fixtures/notification_settings.sql")
    )]
    async fn test_get_notification_settings_for_machine(pool: PgPool) {
        // We'll use the machine ID from the fixtures
        let machine_id = uuid::Uuid::parse_str("dcbf22c7-9d96-47ac-bf06-62d6544e440d").unwrap();

        // First set up the notification settings
        NotificationSettings::set(&pool, 1, true, false, true).await.unwrap();

        // Add a test email
        NotificationSettings::add_email(&pool, 1, "machine_test@example.com").await.unwrap();

        // Get settings for the machine
        let settings = NotificationSettings::get_for_machine(&pool, machine_id).await.unwrap();

        // Verify the settings match what we expect
        assert_eq!(settings.organization_id, 1);
        assert!(settings.email);
        assert!(!settings.telegram);
        assert!(settings.pagerduty);

        // Verify the email was added
        assert!(settings.sendgrid_emails.contains("machine_test@example.com"));
    }

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../fixtures/new_user_registration.sql", "../fixtures/notification_settings.sql")
    )]
    async fn test_get_many_notification_settings_for_client(pool: PgPool) {
        // We'll use the client IDs from the fixtures
        let client_id1 = "0x0101010101010101010101010101010101010101".parse::<Address>().unwrap();
        let client_id2 = "0x0101010101010101010101010101010101010102".parse::<Address>().unwrap();
        let client_ids = vec![client_id1, client_id2];

        // Set up notification settings for both organizations
        NotificationSettings::set(&pool, 1, true, false, true).await.unwrap();
        NotificationSettings::set(&pool, 2, false, true, false).await.unwrap();

        // Add different emails to distinguish them
        NotificationSettings::add_email(&pool, 1, "client1@example.com").await.unwrap();
        NotificationSettings::add_email(&pool, 2, "client2@example.com").await.unwrap();

        // Get settings for both clients
        let settings = NotificationSettings::get_many_for_client(&pool, &client_ids).await.unwrap();

        // We should have two settings objects
        assert_eq!(settings.len(), 2);

        // Find each organization's settings and verify they're correct
        let org1_settings = settings.iter().find(|s| s.organization_id == 1).unwrap();
        let org2_settings = settings.iter().find(|s| s.organization_id == 2).unwrap();

        // Verify org 1 settings
        assert!(org1_settings.email);
        assert!(!org1_settings.telegram);
        assert!(org1_settings.pagerduty);
        assert!(org1_settings.sendgrid_emails.contains("client1@example.com"));

        // Verify org 2 settings
        assert!(!org2_settings.email);
        assert!(org2_settings.telegram);
        assert!(!org2_settings.pagerduty);
        assert!(org2_settings.sendgrid_emails.contains("client2@example.com"));
    }

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../fixtures/new_user_registration.sql", "../fixtures/notification_settings.sql")
    )]
    async fn test_get_many_notification_settings_for_machine(pool: PgPool) {
        // We'll use the machine IDs from the fixtures
        let machine_id1 = uuid::Uuid::parse_str("dcbf22c7-9d96-47ac-bf06-62d6544e440d").unwrap();
        let machine_id2 = uuid::Uuid::parse_str("d160619b-5fb8-4507-b73a-e2f5bd05d477").unwrap();
        let machine_ids = vec![machine_id1, machine_id2];

        // Set up notification settings for both organizations
        NotificationSettings::set(&pool, 1, true, false, true).await.unwrap();
        NotificationSettings::set(&pool, 2, false, true, false).await.unwrap();

        // Add different emails to distinguish them
        NotificationSettings::add_email(&pool, 1, "machine1@example.com").await.unwrap();
        NotificationSettings::add_email(&pool, 2, "machine2@example.com").await.unwrap();

        // Get settings for both machines
        let settings =
            NotificationSettings::get_many_for_machine(&pool, &machine_ids).await.unwrap();

        // We should have two settings objects
        assert_eq!(settings.len(), 2);

        // Find each organization's settings and verify they're correct
        let org1_settings = settings.iter().find(|s| s.organization_id == 1).unwrap();
        let org2_settings = settings.iter().find(|s| s.organization_id == 2).unwrap();

        // Verify org 1 settings
        assert!(org1_settings.email);
        assert!(!org1_settings.telegram);
        assert!(org1_settings.pagerduty);
        assert!(org1_settings.sendgrid_emails.contains("machine1@example.com"));

        // Verify org 2 settings
        assert!(!org2_settings.email);
        assert!(org2_settings.telegram);
        assert!(!org2_settings.pagerduty);
        assert!(org2_settings.sendgrid_emails.contains("machine2@example.com"));
    }
}
