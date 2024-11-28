use crate::error::BackendError;
use ivynet_core::ethers::types::Address;
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct OperatorKey {
    pub id: i64,
    pub organization_id: i64,
    pub name: String,
    pub public_key: Address,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct DbOperatorKey {
    pub id: i64,
    pub organization_id: i64,
    pub name: String,
    pub public_key: String,
}

impl TryFrom<DbOperatorKey> for OperatorKey {
    type Error = BackendError;

    fn try_from(key: DbOperatorKey) -> Result<Self, Self::Error> {
        Ok(OperatorKey {
            id: key.id,
            organization_id: key.organization_id,
            name: key.name,
            public_key: key.public_key.parse::<Address>().map_err(|_| BackendError::BadId)?,
        })
    }
}

impl OperatorKey {
    pub async fn get_all_keys_for_organization(
        pool: &PgPool,
        organization_id: i64,
    ) -> Result<Vec<OperatorKey>, BackendError> {
        let keys = sqlx::query_as!(
           DbOperatorKey,
           "SELECT id, organization_id, name, public_key FROM operator_keys WHERE organization_id = $1",
           organization_id
       )
       .fetch_all(pool)
       .await?;

        keys.into_iter().map(|k| k.try_into()).collect()
    }

    pub async fn get(pool: &PgPool, id: i64) -> Result<Option<OperatorKey>, BackendError> {
        let key = sqlx::query_as!(
            DbOperatorKey,
            "SELECT id, organization_id, name, public_key FROM operator_keys WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await?;

        key.map(|k| k.try_into()).transpose()
    }

    pub async fn create(
        pool: &PgPool,
        organization_id: i64,
        name: &str,
        public_key: &Address,
    ) -> Result<(), BackendError> {
        query!(
            "INSERT INTO operator_keys (organization_id, name, public_key) values ($1, $2, $3)",
            organization_id,
            name,
            public_key.to_string(),
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn change_name(
        pool: &PgPool,
        organization_id: i64,
        public_key: &Address,
        name: &str,
    ) -> Result<(), BackendError> {
        query!(
            "UPDATE operator_keys SET name = $1 WHERE organization_id = $2 AND public_key = $3",
            name,
            organization_id,
            public_key.to_string(),
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(
        pool: &PgPool,
        organization_id: i64,
        public_key: &Address,
    ) -> Result<(), BackendError> {
        query!(
            "DELETE FROM operator_keys WHERE organization_id = $1 AND public_key = $2",
            organization_id,
            public_key.to_string(),
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn purge(pool: &PgPool, organization_id: i64) -> Result<(), BackendError> {
        query!("DELETE FROM operator_keys WHERE organization_id = $1", organization_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
