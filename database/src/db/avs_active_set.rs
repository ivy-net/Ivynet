use crate::error::DatabaseError;
use ivynet_core::{
    ethers::types::{Address, Chain},
    grpc::backend_events::RegistrationEvent,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AvsActiveSet {
    pub directory: Address,
    pub avs: Address,
    pub operator: Address,
    pub chain_id: u64,
    pub active: bool,
    pub block: u64,
    pub log_index: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DbAvsActiveSet {
    pub directory: Vec<u8>,
    pub avs: Vec<u8>,
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
            avs: key.avs.as_bytes().to_vec(),
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
            avs: Address::from_slice(&key.avs),
            operator: Address::from_slice(&key.operator),
            chain_id: key.chain_id as u64,
            active: key.active,
            block: key.block as u64,
            log_index: key.log_index as u64,
        }
    }
}

impl AvsActiveSet {
    pub async fn record_registration_event(
        pool: &sqlx::PgPool,
        event: &RegistrationEvent,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "INSERT INTO avs_active_set (directory, avs, operator, chain_id, active, block, log_index)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (avs, operator, chain_id)
             DO UPDATE SET active = $5, block = $6, log_index = $7",
             event.directory,
             event.avs,
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
        avs: Address,
        operator: Address,
        chain: Chain,
    ) -> Result<bool, DatabaseError> {
        let set = sqlx::query_as!(
            DbAvsActiveSet, r#"SELECT directory, avs, operator, chain_id, active, block, log_index
                            FROM avs_active_set WHERE avs = $1 AND operator = $2 AND chain_id = $3"#,
            avs.as_bytes(),
            operator.as_bytes(),
            (chain as u64) as i64
            ).fetch_optional(pool).await?;

        Ok(set.map(|a| a.active).unwrap_or(false))
    }

    pub async fn get_latest_block(
        pool: &sqlx::PgPool,
        directory: &[u8],
        chain: u64,
    ) -> Result<u64, DatabaseError> {
        if let Some(block) = sqlx::query_scalar!(
            r#"SELECT max(block) FROM avs_active_set WHERE directory = $1 AND chain_id = $2"#,
            directory,
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

#[cfg(test)]
mod scraper_tests {
    use super::*;
    use sqlx::PgPool;

    #[ignore]
    #[sqlx::test]
    async fn test_add_avs_active_set(pool: PgPool) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        std::env::set_var("DATABASE_URL", "postgresql://ivy:secret_ivy@localhost:5432/ivynet");
        let avs = Address::from_slice(&[1; 20]);
        let operator = Address::from_slice(&[3; 20]);

        let event = RegistrationEvent {
            directory: Address::from_slice(&[1; 20]).as_bytes().to_vec(),
            avs: avs.as_bytes().to_vec(),
            address: operator.as_bytes().to_vec(),
            chain_id: 1,
            active: true,
            block_number: 1,
            log_index: 1,
        };

        AvsActiveSet::record_registration_event(&pool, &event).await.unwrap();

        // happy path
        let set = AvsActiveSet::get_active_set(&pool, avs, operator, Chain::Mainnet).await.unwrap();
        assert!(set);

        // sad path
        let set = AvsActiveSet::get_active_set(&pool, operator, avs, Chain::Optimism).await?;
        assert!(!set);
        Ok(())
    }
}
