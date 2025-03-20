use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;

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
    pub fn uuid(&self) -> Uuid {
        let uuid_seed =
            format!("{}-{:?}-{}", self.organization_id, self.settings_type, self.settings_value);
        Uuid::new_v5(&Uuid::NAMESPACE_URL, uuid_seed.as_bytes())
    }

    // Create or update a service setting. Returns the UUID of the setting if it was created or
    // None if it already existed.
    pub async fn create(
        pool: &PgPool,
        org_id: u64,
        settings_type: ServiceType,
        value: &str,
    ) -> Result<Option<Uuid>, DatabaseError> {
        // Skip creation for empty values as they represent valid empty states
        if value.trim().is_empty() {
            return Ok(None);
        }

        let service_setting = Self {
            organization_id: org_id as i64,
            settings_type,
            settings_value: value.to_string(),
            created_at: None,
        };

        let uuid = service_setting.uuid();

        // Validate that we have a valid UUID before attempting insertion
        if uuid.is_nil() {
            return Err(DatabaseError::InvalidInput("Generated UUID is nil".to_string()));
        }

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

#[cfg(test)]
mod notification_settings_tests {
    use super::*;
    use sqlx::PgPool;

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
}
