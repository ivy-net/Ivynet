use chrono::NaiveDateTime;
use ivynet_error::ethers::types::{Address, Chain};
use ivynet_node_type::NodeType;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Avs {
    pub machine_id: Uuid,
    pub avs_name: String, //GIVEN BY THE USER OR A DEFAULT
    pub avs_type: NodeType,
    pub avs_version: String,
    pub chain: Option<Chain>,
    pub version_hash: String,
    pub operator_address: Option<Address>,
    pub active_set: bool,
    pub metrics_alive: bool,
    pub node_running: bool,
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
    pub metrics_alive: bool,
    pub node_running: bool,
    pub version_hash: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Debug, thiserror::Error)]
enum AvsError {
    #[error(transparent)]
    UnknownAvs(#[from] ivynet_node_type::NodeTypeError),

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
            avs_version: db_avs.avs_version,
            operator_address: db_avs.operator_address.map(|a| Address::from_slice(&a)),
            active_set: db_avs.active_set,
            metrics_alive: db_avs.metrics_alive,
            node_running: db_avs.node_running,
            version_hash: db_avs.version_hash,
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
    ) -> Result<Vec<Avs>, DatabaseError> {
        let avses: Vec<DbAvs> = sqlx::query_as!(
            DbAvs,
            "SELECT machine_id, avs_name, avs_type, chain, avs_version, operator_address, version_hash, active_set, metrics_alive, node_running, created_at, updated_at FROM avs WHERE machine_id = $1",
            Some(machine_id)
        )
        .fetch_all(pool)
        .await?;

        Ok(avses.into_iter().filter_map(|e| Avs::try_from(e).ok()).collect())
    }

    pub async fn get_avs_list_count(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
    ) -> Result<i64, DatabaseError> {
        let count: Option<i64> =
            sqlx::query_scalar!("SELECT COUNT(*) FROM avs WHERE machine_id = $1", Some(machine_id))
                .fetch_one(pool)
                .await?;

        if let Some(count) = count {
            return Ok(count);
        }

        Ok(0)
    }

    pub async fn get_org_avs_list(
        pool: &sqlx::PgPool,
        organization_id: i64,
    ) -> Result<Vec<Avs>, DatabaseError> {
        let avses: Vec<DbAvs> = sqlx::query_as!(
            DbAvs,
            r#"
            SELECT
                avs.machine_id,
                avs.avs_name,
                avs.avs_type,
                avs.chain,
                avs.avs_version,
                avs.operator_address,
                avs.version_hash,
                avs.active_set,
                avs.metrics_alive,
                avs.node_running,
                avs.created_at,
                avs.updated_at
            FROM avs
            JOIN machine ON avs.machine_id = machine.machine_id
            JOIN client ON machine.client_id = client.client_id
            WHERE client.organization_id = $1
            "#,
            organization_id
        )
        .fetch_all(pool)
        .await?;
        Ok(avses.into_iter().filter_map(|e| Avs::try_from(e).ok()).collect())
    }

    pub async fn get_machines_avs(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
        avs_name: &str,
    ) -> Result<Option<Avs>, DatabaseError> {
        let avs: Option<DbAvs> = sqlx::query_as!(
            DbAvs,
            "SELECT machine_id, avs_name, avs_type, chain, avs_version, operator_address, active_set, metrics_alive, node_running, version_hash, created_at, updated_at FROM avs WHERE machine_id = $1 AND avs_name = $2",
            Some(machine_id),
            avs_name
        )
        .fetch_optional(pool)
        .await?;

        avs.map(|avs| Avs::try_from(avs).map_err(|_| DatabaseError::BadId)).transpose()
    }

    pub async fn get_by_operator_address(
        pool: &sqlx::PgPool,
        operator_id: &Address,
    ) -> Result<Vec<Avs>, DatabaseError> {
        let avses: Vec<DbAvs> = sqlx::query_as!(
            DbAvs,
            "SELECT machine_id, avs_name, avs_type, chain, avs_version, operator_address, active_set, metrics_alive, node_running, version_hash, created_at, updated_at FROM avs WHERE operator_address = $1",
            operator_id.as_bytes()
        )
        .fetch_all(pool)
        .await?;

        Ok(avses.into_iter().filter_map(|e| Avs::try_from(e).ok()).collect())
    }

    pub async fn get_operator_avs_list(
        pool: &sqlx::PgPool,
        operator_id: &Address,
    ) -> Result<Vec<Avs>, DatabaseError> {
        let avses: Vec<DbAvs> = sqlx::query_as!(
            DbAvs,
            "SELECT machine_id, avs_name, avs_type, chain, avs_version, operator_address, active_set, metrics_alive, node_running, version_hash, created_at, updated_at FROM avs WHERE operator_address = $1",
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
        version_hash: &str,
    ) -> Result<(), DatabaseError> {
        let now = chrono::Utc::now().naive_utc();

        let mut tx = pool.begin().await?;

        // The CASE statement in the UPDATE clause only updates avs_type
        // when the existing type is 'unknown'
        // If we get a bug where version hash doesn't match node_type, its because
        // the user assigned the wrong node_type
        let result = sqlx::query!(
            "INSERT INTO avs (
                avs_name, machine_id, avs_type, avs_version,
                active_set, operator_address, version_hash,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (machine_id, avs_name) DO UPDATE
            SET avs_type = CASE
                    WHEN avs.avs_type = 'unknown' THEN EXCLUDED.avs_type
                    ELSE avs.avs_type
                END,
                updated_at = EXCLUDED.updated_at,
                version_hash = EXCLUDED.version_hash",
            avs_name,
            machine_id,
            avs_type.to_string(),
            "0.0.0",
            false,
            Option::<Vec<u8>>::None,
            version_hash,
            now,
            now
        )
        .execute(&mut *tx)
        .await;

        match result {
            Ok(pg_result) => {
                if pg_result.rows_affected() == 1 {
                    tx.commit().await?;
                    Ok(())
                } else {
                    tx.rollback().await?;
                    Err(DatabaseError::DataIntegrityError(
                        "Unexpected number of rows affected".into(),
                    ))
                }
            }
            Err(e) => {
                tx.rollback().await?;
                Err(DatabaseError::from(e))
            }
        }
    }

    pub async fn delete_avs_data(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
        avs_name: &str,
    ) -> Result<(), DatabaseError> {
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
    ) -> Result<(), DatabaseError> {
        sqlx::query!("DELETE FROM avs WHERE machine_id = $1", machine_id).execute(pool).await?;
        Ok(())
    }

    pub async fn update_operator_address(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
        avs_name: &str,
        operator_address: Option<Address>,
    ) -> Result<(), DatabaseError> {
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

    pub async fn update_chain(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
        avs_name: &str,
        chain: Chain,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "UPDATE avs SET chain = $1 WHERE machine_id = $2 AND avs_name = $3",
            chain.to_string(),
            machine_id,
            avs_name
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_active_set(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
        avs_name: &str,
        active_set: bool,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "UPDATE avs SET active_set = $1 WHERE machine_id = $2 AND avs_name = $3",
            active_set,
            machine_id,
            avs_name
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_node_type(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
        avs_name: &str,
        node_type: &NodeType,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "UPDATE avs SET avs_type = $1 WHERE machine_id = $2 AND avs_name = $3",
            node_type.to_string(),
            machine_id,
            avs_name
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_version(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
        avs_name: &str,
        version: &str,
        image_digest: &str,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "UPDATE avs SET avs_version = $1, version_hash = $2 WHERE machine_id = $3 AND avs_name = $4",
            version,
            image_digest,
            machine_id,
            avs_name
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_metrics_alive(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
        avs_name: &str,
        metrics_alive: bool,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "UPDATE avs SET metrics_alive = $1 WHERE machine_id = $2 AND avs_name = $3",
            metrics_alive,
            machine_id,
            avs_name
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_node_running(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
        avs_name: &str,
        running: bool,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "UPDATE avs SET node_running = $1 WHERE machine_id = $2 AND avs_name = $3",
            running,
            machine_id,
            avs_name
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_name(
        pool: &sqlx::PgPool,
        machine_id: Uuid,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "UPDATE avs SET avs_name = $1 WHERE machine_id = $2 AND avs_name = $3",
            new_name,
            machine_id,
            old_name
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
