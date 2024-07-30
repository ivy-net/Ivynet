use crate::error::BackendError;

use chrono::{NaiveDateTime, Utc};
use ivynet_core::ethers::types::Address;
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};
use utoipa::ToSchema;

use super::{
    node::{DbNode, Node},
    Organization,
};

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Deserialize, Serialize, ToSchema)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum Role {
    Owner,
    Admin,
    User,
    Reader,
}

impl Role {
    pub fn is_admin(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }

    pub fn can_write(&self) -> bool {
        match self {
            Role::Owner | Role::Admin | Role::User => true,
            Role::Reader => false,
        }
    }
}

#[derive(sqlx::FromRow, Deserialize, Serialize, Clone, Debug)]
pub struct Account {
    pub user_id: i64,
    pub organization_id: i64,
    pub email: String,
    pub password: String,
    pub role: Role,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl Account {
    pub async fn get_all(pool: &PgPool) -> Result<Vec<Account>, BackendError> {
        let accounts = sqlx::query_as!(
            Account,
            r#"SELECT user_id, organization_id, email, password, role AS "role!: Role", created_at, updated_at FROM account"#
        )
            .fetch_all(pool) // -> Vec<Country>
            .await?;

        Ok(accounts)
    }

    pub async fn verify(
        pool: &PgPool,
        email: &str,
        password: &str,
    ) -> Result<Account, BackendError> {
        let account = sqlx::query_as!(
        Account,
        r#"SELECT user_id, organization_id, email, password, role AS "role!: Role", created_at, updated_at FROM account WHERE email = $1 AND password = $2"#,
        email, sha256::digest(password)
    )
    .fetch_one(pool)
    .await?;

        Ok(account)
    }

    pub async fn set_password(
        &self,
        pool: &PgPool,
        password: &str,
    ) -> Result<Account, BackendError> {
        let account = sqlx::query_as!(
            Account,
            r#"UPDATE account SET password = $1 WHERE email = $2
                    RETURNING user_id, organization_id, email, password, role AS "role: _", created_at, updated_at"#,
            sha256::digest(password),
            self.email,
        )
        .fetch_one(pool)
        .await?;
        Ok(account)
    }

    pub async fn get(pool: &PgPool, id: u64) -> Result<Account, BackendError> {
        let account = sqlx::query_as!(
        Account,
        r#"SELECT user_id, organization_id, email, password, role AS "role!: Role", created_at, updated_at FROM account WHERE user_id = $1"#,
        id as i64
    )
    .fetch_one(pool)
    .await?;

        Ok(account)
    }

    pub async fn exists(pool: &PgPool, email: &str) -> Result<bool, BackendError> {
        match sqlx::query_as!(
            Account,
            r#"SELECT user_id, organization_id, email, password, role AS "role!: Role", created_at, updated_at FROM account WHERE email = $1"#,
            email    )
            .fetch_one(pool)
            .await
            {
                Ok(_) => Ok(true),
                Err(sqlx::Error::RowNotFound) => Ok(false),
                Err(err) => Err(err.into())

            }
    }

    pub async fn nodes(&self, pool: &PgPool) -> Result<Vec<Node>, BackendError> {
        DbNode::get_all_for_account(pool, self).await
    }

    pub async fn attach_node(&self, pool: &PgPool, node_id: &Address) -> Result<(), BackendError> {
        DbNode::create(pool, self, node_id).await
    }

    pub async fn new(
        pool: &PgPool,
        organization: &Organization,
        email: &str,
        password: &str,
        role: Role,
    ) -> Result<Account, BackendError> {
        let now: NaiveDateTime = Utc::now().naive_utc();

        let account = sqlx::query_as!(
            Account,
            r#"INSERT INTO account (organization_id, email, password, role, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    RETURNING user_id, organization_id, email, password, role AS "role: _", created_at, updated_at"#,
            organization.organization_id,
            email,
            sha256::digest(password),
            role as Role,
            now,
            now
        )
        .fetch_one(pool)
        .await?;
        Ok(account)
    }

    pub async fn delete(&self, pool: &PgPool) -> Result<(), BackendError> {
        query!("DELETE FROM account WHERE email = $1", self.email).execute(pool).await?;
        Ok(())
    }

    pub async fn purge(pool: &PgPool) -> Result<(), BackendError> {
        query!("DELETE FROM account").execute(pool).await?;
        Ok(())
    }
}
