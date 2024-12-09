use crate::error::BackendError;
use ivynet_core::{
    ethers::types::{Address, Chain},
    grpc::backend_events::Event,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AvsActiveSet {
    pub directory: Address,
    pub operator: Address,
    pub chain_id: u64,
    pub active: bool,
    pub block: u64,
    pub log_index: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct DbAvsActiveSet {
    pub directory: Vec<u8>,
    pub operator: Vec<u8>,
    pub chain_id: i64,
    pub active: bool,
    pub block: i64,
    pub log_index: i64,
}

impl From<AvsActiveSet> for DbAvsActiveSet {
    fn from(key: AvsActiveSet) -> Self {
        DbAvsActiveSet {
            directory: key.directory.as_bytes().to_vec(),
            operator: key.operator.as_bytes().to_vec(),
            chain_id: key.chain_id as i64,
            active: key.active,
            block: key.block as i64,
            log_index: key.log_index as i64,
        }
    }
}

impl From<DbAvsActiveSet> for AvsActiveSet {
    fn from(key: DbAvsActiveSet) -> Self {
        AvsActiveSet {
            directory: Address::from_slice(&key.directory),
            operator: Address::from_slice(&key.operator),
            chain_id: key.chain_id as u64,
            active: key.active,
            block: key.block as u64,
            log_index: key.log_index as u64,
        }
    }
}

impl AvsActiveSet {
    pub async fn record_event(pool: &sqlx::PgPool, event: &Event) -> Result<(), BackendError> {
        sqlx::query!(
            "INSERT INTO avs_active_set (directory, operator, chain_id, active, block, log_index)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (directory, operator, chain_id)
             DO UPDATE SET active = $4, block = $5, log_index = $6",
            event.directory,
            event.address,
            event.chain_id as i64,
            event.active,
            event.block_number as i64,
            event.log_index as i64
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_active_set(
        pool: &sqlx::PgPool,
        directory: Address,
        operator: Address,
        chain: Chain,
    ) -> Result<bool, BackendError> {
        let set = sqlx::query_as!(
            DbAvsActiveSet, r#"SELECT directory, operator, chain_id, active, block, log_index
                            FROM avs_active_set WHERE directory = $1 AND operator = $2 AND chain_id = $3"#,
            directory.as_bytes(),
            operator.as_bytes(),
            (chain as u64) as i64
            ).fetch_optional(pool).await?;

        Ok(set.map(|a| a.active).unwrap_or(false))
    }

    pub async fn get_latest_block(pool: &sqlx::PgPool, chain: u64) -> Result<u64, BackendError> {
        if let Some(block) = sqlx::query_scalar!(
            r#"SELECT max(block) FROM avs_active_set WHERE chain_id = $1"#,
            (chain as u64) as i64
        )
        .fetch_optional(pool)
        .await?
        {
            Ok(block.unwrap_or(0) as u64)
        } else {
            Ok(0)
        }
    }
}
