use chrono::{NaiveDateTime, Utc};
use ivynet_core::ethers::types::Address;
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};
use utoipa::ToSchema;

use crate::error::DatabaseError;

use super::Account;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Client {
    pub client_id: Address,
    pub organization_id: i64,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Clone, Debug)]
struct DbClient {
    pub client_id: Vec<u8>,
    pub organization_id: i64,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl From<DbClient> for Client {
    fn from(value: DbClient) -> Self {
        Self {
            client_id: Address::from_slice(&value.client_id),
            organization_id: value.organization_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl Client {
    pub async fn get_all_for_account(
        pool: &PgPool,
        account: &Account,
    ) -> Result<Vec<Client>, DatabaseError> {
        let clients = sqlx::query_as!(
            DbClient,
            "SELECT client_id, organization_id, created_at, updated_at FROM client WHERE organization_id = $1",
            account.organization_id
        )
        .fetch_all(pool)
        .await?;

        Ok(clients.into_iter().map(|n| n.into()).collect())
    }

    pub async fn get(pool: &PgPool, client_id: &Address) -> Result<Option<Client>, DatabaseError> {
        let client: Option<DbClient> = sqlx::query_as!(
            DbClient,
            "SELECT client_id, organization_id, created_at, updated_at FROM client WHERE client_id = $1",
            Some(client_id.as_bytes())
        )
        .fetch_optional(pool)
        .await?;

        Ok(client.map(|m| m.into()))
    }

    pub async fn create(
        pool: &PgPool,
        account: &Account,
        client_id: &Address,
    ) -> Result<(), DatabaseError> {
        let now: NaiveDateTime = Utc::now().naive_utc();

        query!(
            "INSERT INTO client (client_id, organization_id, created_at, updated_at) values ($1, $2, $3, $4)",
            Some(client_id.as_bytes()),
            Some(account.organization_id),
            Some(now),
            Some(now)
        )
        .execute(pool)
        .await?;
        Ok(())
    }
    pub async fn delete(pool: &PgPool, client_id: &Address) -> Result<(), DatabaseError> {
        query!("DELETE FROM client WHERE client_id = $1", client_id.as_bytes())
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn purge(pool: &PgPool) -> Result<(), DatabaseError> {
        query!("DELETE FROM client").execute(pool).await?;
        Ok(())
    }
}
