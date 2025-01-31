use chrono::{NaiveDateTime, Utc};
use ivynet_core::ethers::types::Address;
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Machine {
    pub machine_id: Uuid,
    pub name: String,
    pub client_id: Address,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Clone, Debug)]
struct DbMachine {
    pub machine_id: Uuid,
    pub name: String,
    pub client_id: Vec<u8>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl From<DbMachine> for Machine {
    fn from(value: DbMachine) -> Self {
        Self {
            machine_id: value.machine_id,
            name: value.name.clone(),
            client_id: Address::from_slice(&value.client_id),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
impl Machine {
    pub async fn get(pool: &PgPool, machine_id: Uuid) -> Result<Option<Machine>, DatabaseError> {
        let machines = sqlx::query_as!(
            DbMachine,
            "SELECT machine_id, name, client_id, created_at, updated_at FROM machine WHERE machine_id = $1",
            machine_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(machines.map(|m| m.into()))
    }

    pub async fn get_all_for_client_id(
        pool: &PgPool,
        client_id: &Address,
    ) -> Result<Vec<Machine>, DatabaseError> {
        let machines = sqlx::query_as!(
            DbMachine,
            "SELECT machine_id, name, client_id, created_at, updated_at FROM machine WHERE client_id = $1",
            Some(client_id.as_bytes())
        )
        .fetch_all(pool)
        .await?;

        Ok(machines.into_iter().map(|n| n.into()).collect())
    }

    pub async fn is_owned_by(
        pool: &PgPool,
        client_id: &Address,
        machine_id: Uuid,
    ) -> Result<bool, DatabaseError> {
        let machines = sqlx::query_as!(
            DbMachine,
            "SELECT machine_id, name, client_id, created_at, updated_at FROM machine WHERE client_id = $1 AND machine_id = $2",
            Some(client_id.as_bytes()),
            Some(machine_id)
        )
        .fetch_all(pool)
        .await?;
        Ok(!machines.is_empty())
    }

    pub async fn create(
        pool: &PgPool,
        client_id: &Address,
        name: &str,
        machine_id: Uuid,
    ) -> Result<(), DatabaseError> {
        let now: NaiveDateTime = Utc::now().naive_utc();

        query!(
            "INSERT INTO machine (machine_id, name, client_id, created_at, updated_at) values ($1, $2, $3, $4, $5)",
            Some(machine_id),
            Some(name),
            Some(client_id.as_bytes()),
            Some(now),
            Some(now)
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn set_name(&self, pool: &PgPool, name: &str) -> Result<(), DatabaseError> {
        let now: NaiveDateTime = Utc::now().naive_utc();
        query!(
            "UPDATE machine SET name = $2, updated_at = $3 WHERE machine_id = $1",
            self.machine_id,
            name,
            Some(now)
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, pool: &PgPool) -> Result<(), DatabaseError> {
        query!("DELETE FROM machine WHERE machine_id = $1", self.machine_id).execute(pool).await?;
        Ok(())
    }

    pub async fn purge(pool: &PgPool) -> Result<(), DatabaseError> {
        query!("DELETE FROM machine").execute(pool).await?;
        Ok(())
    }
}
