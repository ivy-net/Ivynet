use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};

use crate::error::BackendError;

use super::{verification::Verification, Account, Role};

#[derive(sqlx::FromRow, Deserialize, Serialize, Clone, Debug)]
pub struct Organization {
    pub organization_id: i64,
    pub name: String,
    pub verified: bool,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl Organization {
    pub async fn new(pool: &PgPool, name: &str, verified: bool) -> Result<Organization, BackendError> {
        let now: NaiveDateTime = Utc::now().naive_utc();
        let org = sqlx::query_as!(
            Organization,
            r#"INSERT INTO organization (name, verified, created_at, updated_at)
                    VALUES ($1, $2, $3, $4)
                    RETURNING organization_id, name, verified, created_at, updated_at"#,
            name,
            verified,
            now,
            now
        )
        .fetch_one(pool)
        .await?;
        Ok(org)
    }

    pub async fn get(pool: &PgPool, id: u64) -> Result<Organization, BackendError> {
        let org = sqlx::query_as!(
            Organization,
            r#"SELECT organization_id, name, verified, created_at, updated_at FROM organization
                WHERE organization_id = $1"#,
            id as i64
        )
        .fetch_one(pool)
        .await?;
        Ok(org)
    }

    pub async fn attach_admin(&self, pool: &PgPool, email: &str, password: &str) -> Result<Account, BackendError> {
        Account::new(pool, self, email, password, Role::Admin).await
    }

    pub async fn invite(&self, pool: &PgPool, email: &str, role: Role) -> Result<Verification, BackendError> {
        let account = Account::new(pool, self, email, "", role).await?;

        let verification =
            Verification::new(pool, super::verification::VerificationType::User, account.user_id).await?;

        Ok(verification)
    }

    pub async fn verify(&mut self, pool: &PgPool) -> Result<(), BackendError> {
        self.verified = true;
        query!("UPDATE organization SET verified = true WHERE organization_id = $1", self.organization_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn purge(pool: &PgPool) -> Result<(), BackendError> {
        query!("DELETE FROM organization").execute(pool).await?;
        Ok(())
    }
}
