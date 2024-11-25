use chrono::NaiveDateTime;
use ivynet_core::{
    ethers::types::{Address, Chain},
    node_type::NodeType,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::BackendError;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Avs {
    pub machine_id: Uuid,
    pub avs_name: String, //GIVEN BY THE USER OR A DEFAULT
    pub avs_type: NodeType,
    pub avs_version: Version,
    pub chain: Option<Chain>,
    pub operator_address: Option<Address>,
    pub active_set: bool,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Clone, Debug)]
struct DbAvs {
    pub machine_id: Vec<u8>,
    pub avs_name: String,
    pub avs_type: String,
    pub chain: Option<String>,
    pub avs_version: String,
    pub operator_address: Option<Vec<u8>>,
    pub active_set: bool,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, thiserror::Error)]
enum AvsError {
    #[error(transparent)]
    UnknownAvs(#[from] ivynet_core::node_type::NodeTypeError),

    #[error(transparent)]
    BadVersion(#[from] semver::Error),

    #[error(transparent)]
    WrongMachineId(#[from] uuid::Error),
}

impl TryFrom<DbAvs> for Avs {
    type Error = AvsError;
    fn try_from(db_avs: DbAvs) -> Result<Self, Self::Error> {
        Ok(Avs {
            machine_id: Uuid::from_slice(&db_avs.machine_id)?,
            avs_type: NodeType::from(db_avs.avs_type.as_str()),
            avs_name: db_avs.avs_name,
            avs_version: Version::parse(&db_avs.avs_version)?,
            operator_address: db_avs.operator_address.map(|a| Address::from_slice(&a)),
            active_set: db_avs.active_set,
            created_at: db_avs.created_at,
            updated_at: db_avs.updated_at,
            chain: db_avs.chain.and_then(|c| c.parse::<Chain>().ok()),
        })
    }
}

impl Avs {
    pub async fn get_machines_avs_list(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
    ) -> Result<Vec<Avs>, BackendError> {
        let avses: Vec<DbAvs> = sqlx::query_as!(
            DbAvs,
            "SELECT machine_id, avs_name, avs_type, chain, avs_version, operator_address, active_set, created_at, updated_at FROM avs WHERE machine_id = $1",
            Some(machine_id)
        )
        .fetch_all(pool)
        .await?;

        Ok(avses.into_iter().filter_map(|e| Avs::try_from(e).ok()).collect())
    }

    pub async fn get_machines_avs(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
        avs_name: &str,
    ) -> Result<Option<Avs>, BackendError> {
        let avs: Option<DbAvs> = sqlx::query_as!(
            DbAvs,
            "SELECT machine_id, avs_name, avs_type, chain, avs_version, operator_address, active_set, created_at, updated_at FROM avs WHERE machine_id = $1 AND avs_name = $2",
            Some(machine_id),
            avs_name
        )
        .fetch_optional(pool)
        .await?;

        avs.map(|avs| Avs::try_from(avs).map_err(|_| BackendError::BadId)).transpose()
    }

    pub async fn get_operator_avs_list(
        pool: &sqlx::PgPool,
        operator_id: &Address,
    ) -> Result<Vec<Avs>, BackendError> {
        let avses: Vec<DbAvs> = sqlx::query_as!(
            DbAvs,
            "SELECT machine_id, avs_name, avs_type, chain, avs_version, operator_address, active_set, created_at, updated_at FROM avs WHERE operator_address = $1",
            operator_id.as_bytes()
        )
        .fetch_all(pool)
        .await?;

        Ok(avses.into_iter().filter_map(|e| Avs::try_from(e).ok()).collect())
    }

    pub async fn record_avs_data_from_client(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
        avs_name: &str,
        avs_type: &NodeType,
        avs_version: &Version,
    ) -> Result<(), BackendError> {
        let now = chrono::Utc::now().naive_utc();

        sqlx::query!(
            "INSERT INTO avs (avs_name, machine_id, avs_type, avs_version, active_set, operator_address, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (machine_id, avs_name)
             DO UPDATE SET avs_version = EXCLUDED.avs_version, updated_at = $8",
            avs_name.to_string(),
            machine_id,
            avs_type.clone().to_string(),
            avs_version.to_string(),
            false,
            Option::<Vec<u8>>::None,
            now,
            now
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete_avs_data(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
        avs_name: &str,
    ) -> Result<(), BackendError> {
        sqlx::query!(
            "DELETE FROM avs WHERE avs_name = $1 AND machine_id = $2",
            avs_name.to_string(),
            machine_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete_all_machine_data(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
    ) -> Result<(), BackendError> {
        sqlx::query!("DELETE FROM avs WHERE machine_id = $1", machine_id).execute(pool).await?;
        Ok(())
    }

    pub async fn update_operator_address(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
        avs_name: &str,
        operator_address: Option<Address>,
    ) -> Result<(), BackendError> {
        sqlx::query!(
            "UPDATE avs
             SET operator_address = $1
             WHERE machine_id = $2 AND avs_name = $3",
            operator_address.map(|addr| addr.as_bytes().to_vec()),
            machine_id,
            avs_name
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
