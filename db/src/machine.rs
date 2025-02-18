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
    pub client_version: Option<String>,
}

#[derive(Clone, Debug)]
struct DbMachine {
    pub machine_id: Uuid,
    pub name: String,
    pub client_id: Vec<u8>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub client_version: Option<String>,
}

impl From<DbMachine> for Machine {
    fn from(value: DbMachine) -> Self {
        Self {
            machine_id: value.machine_id,
            name: value.name.clone(),
            client_id: Address::from_slice(&value.client_id),
            created_at: value.created_at,
            updated_at: value.updated_at,
            client_version: value.client_version,
        }
    }
}
impl Machine {
    pub async fn get(pool: &PgPool, machine_id: Uuid) -> Result<Option<Machine>, DatabaseError> {
        let machines = sqlx::query_as!(
            DbMachine,
            "SELECT machine_id, name, client_id, created_at, updated_at, client_version FROM machine WHERE machine_id = $1",
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
            "SELECT machine_id, name, client_id, created_at, updated_at, client_version FROM machine WHERE client_id = $1",
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
            "SELECT machine_id, name, client_id, created_at, updated_at, client_version FROM machine WHERE client_id = $1 AND machine_id = $2",
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
        client_version: Option<String>,
    ) -> Result<(), DatabaseError> {
        let now: NaiveDateTime = Utc::now().naive_utc();

        query!(
            "INSERT INTO machine (machine_id, name, client_id, created_at, updated_at, client_version) values ($1, $2, $3, $4, $5, $6)",
            Some(machine_id),
            Some(name),
            Some(client_id.as_bytes()),
            Some(now),
            Some(now),
            client_version
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

    pub async fn update_client_version(
        pool: &PgPool,
        machine_id: &Uuid,
        client_version: &str,
    ) -> Result<(), DatabaseError> {
        query!(
            "UPDATE machine SET client_version = $2 WHERE machine_id = $1",
            machine_id,
            client_version
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_organization_id(pool: &PgPool, machine_id: Uuid) -> Result<i64, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            SELECT c.organization_id as "organization_id!"
            FROM machine m
            JOIN client c ON c.client_id = m.client_id
            WHERE m.machine_id = $1
            "#,
            machine_id
        )
        .fetch_one(pool)
        .await?;

        Ok(row.organization_id)
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
