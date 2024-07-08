use crate::error::BackendError;

use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "verification_kind", rename_all = "lowercase")]
pub enum VerificationType {
    Organization,
    User,
}

#[derive(sqlx::FromRow, Deserialize, Serialize, Clone, Debug)]
pub struct Verification {
    pub verification_id: Uuid,
    pub associated_id: i64,
    pub verification_type: VerificationType,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl Verification {
    pub async fn get_all(pool: &PgPool) -> Result<Vec<Verification>, BackendError> {
        let verifications = sqlx::query_as!(
            Verification,
            r#"SELECT verification_id, associated_id, verification_type AS "verification_type!: VerificationType", created_at, updated_at FROM verification"#
        )
            .fetch_all(pool) // -> Vec<Country>
            .await?;

        Ok(verifications)
    }

    pub async fn get(pool: &PgPool, id: Uuid) -> Result<Verification, BackendError> {
        let verification = sqlx::query_as!(
            Verification,
            r#"SELECT verification_id, associated_id, verification_type AS "verification_type!: VerificationType", created_at, updated_at FROM verification WHERE verification_id = $1"#,
            id
        )
            .fetch_one(pool)
            .await?;

        Ok(verification)
    }

    pub async fn new(
        pool: &PgPool,
        verification_type: VerificationType,
        associated_id: i64,
    ) -> Result<Verification, BackendError> {
        let now: NaiveDateTime = Utc::now().naive_utc();

        let verification_id = Uuid::new_v4();
        let verification = sqlx::query_as!(
            Verification,
            r#"INSERT INTO verification (verification_id, associated_id, verification_type, created_at, updated_at) 
                    VALUES ($1, $2, $3, $4, $5) 
                    RETURNING verification_id, associated_id, verification_type AS "verification_type: _", created_at, updated_at"#,
            verification_id,
            associated_id,
            verification_type as VerificationType,
            now,
            now
        )
        .fetch_one(pool)
        .await?;
        Ok(verification)
    }

    pub async fn delete(&self, pool: &PgPool) -> Result<(), BackendError> {
        query!("DELETE FROM verification WHERE verification_id = $1", self.verification_id).execute(pool).await?;
        Ok(())
    }
}
