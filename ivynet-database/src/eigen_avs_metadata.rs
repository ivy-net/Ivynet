use chrono::NaiveDateTime;
use ethers::types::Address;
use sqlx::PgPool;
use thiserror::Error;
use tracing::{debug, error, info};

use crate::error::DatabaseError;

/// Represents an AVS metadata entry from Eigen's AVS Directory
#[derive(Debug, Clone)]
pub struct EigenAvsMetadata {
    pub id: Option<i32>,
    pub address: Address,
    pub block_number: i64,
    pub log_index: i32,
    pub metadata_uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub website: Option<String>,
    pub logo: Option<String>,
    pub twitter: Option<String>,
    pub created_at: Option<NaiveDateTime>,
}

/// Database representation of AVS metadata
#[derive(Debug, Clone)]
struct DbEigenAvsMetadata {
    pub id: Option<i32>,
    pub address: String,
    pub block_number: i64,
    pub log_index: i32,
    pub metadata_uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub website: Option<String>,
    pub logo: Option<String>,
    pub twitter: Option<String>,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct MetadataContent {
    pub name: Option<String>,
    pub description: Option<String>,
    pub website: Option<String>,
    pub logo: Option<String>,
    pub twitter: Option<String>,
}

#[derive(Debug, Error)]
enum EigenAvsMetadataError {
    #[error("Invalid Ethereum address format")]
    InvalidAddressFormat,
}

impl TryFrom<DbEigenAvsMetadata> for EigenAvsMetadata {
    type Error = EigenAvsMetadataError;

    fn try_from(db_metadata: DbEigenAvsMetadata) -> Result<Self, Self::Error> {
        let address = db_metadata
            .address
            .parse::<Address>()
            .map_err(|_| EigenAvsMetadataError::InvalidAddressFormat)?;

        Ok(EigenAvsMetadata {
            id: db_metadata.id,
            address,
            block_number: db_metadata.block_number,
            log_index: db_metadata.log_index,
            metadata_uri: db_metadata.metadata_uri,
            name: db_metadata.name,
            description: db_metadata.description,
            website: db_metadata.website,
            logo: db_metadata.logo,
            twitter: db_metadata.twitter,
            created_at: db_metadata.created_at,
        })
    }
}

impl EigenAvsMetadata {
    /// Insert new AVS metadata into the database
    pub async fn insert(
        pool: &PgPool,
        address: Address,
        block_number: i64,
        log_index: i32,
        metadata_uri: String,
        metadata_content: MetadataContent,
    ) -> Result<(), DatabaseError> {
        debug!(
            "Inserting AVS metadata for address: {}, block: {}, log_index: {}",
            address, block_number, log_index
        );

        let address_str = format!("{:?}", address);

        let result = sqlx::query!(
            r#"
            INSERT INTO eigen_avs_metadata (
                address, block_number, log_index, metadata_uri, name, description, website, logo, twitter, created_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, NOW()
            )
            "#,
            address_str,
            block_number,
            log_index,
            metadata_uri,
            metadata_content.name,
            metadata_content.description,
            metadata_content.website,
            metadata_content.logo,
            metadata_content.twitter,
        )
        .execute(pool)
        .await;

        match result {
            Ok(_) => {
                info!(
                    "Successfully inserted AVS metadata for address: {}, block: {}, log_index: {}",
                    address, block_number, log_index
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    "Failed to insert AVS metadata for address: {}, block: {}, log_index: {}, error: {}",
                    address, block_number, log_index, e
                );
                Err(DatabaseError::SqlxError(e))
            }
        }
    }

    /// Get the latest metadata for a specific metadata URI
    pub async fn get_latest_for_metadata_uri(
        pool: &PgPool,
        metadata_uri: &str,
    ) -> Result<Option<EigenAvsMetadata>, DatabaseError> {
        debug!("Getting latest metadata for URI: {}", metadata_uri);

        let result = sqlx::query_as!(
            DbEigenAvsMetadata,
            r#"
            SELECT
                id, address, block_number, log_index, metadata_uri,
                name, description, website, logo, twitter, created_at
            FROM eigen_avs_metadata
            WHERE metadata_uri = $1
            ORDER BY block_number DESC, log_index DESC
            LIMIT 1
            "#,
            metadata_uri,
        )
        .fetch_optional(pool)
        .await;

        match result {
            Ok(Some(db_metadata)) => {
                let metadata = EigenAvsMetadata::try_from(db_metadata)
                    .map_err(|e| DatabaseError::FailedConversion(e.to_string()))?;
                Ok(Some(metadata))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                error!("Failed to get latest metadata for URI: {}, error: {}", metadata_uri, e);
                Err(DatabaseError::SqlxError(e))
            }
        }
    }

    /// Get all metadata entries for a specific AVS address
    pub async fn get_all_with_same_metadata_uri(
        pool: &PgPool,
        metadata_uri: String,
    ) -> Result<Vec<EigenAvsMetadata>, DatabaseError> {
        debug!("Getting all metadata for metadata_uri: {}", metadata_uri);

        let result = sqlx::query_as!(
            DbEigenAvsMetadata,
            r#"
            SELECT
                id, address, block_number, log_index, metadata_uri,
                name, description, website, logo, twitter, created_at
            FROM eigen_avs_metadata
            WHERE metadata_uri = $1
            ORDER BY block_number DESC, log_index DESC
            "#,
            metadata_uri,
        )
        .fetch_all(pool)
        .await;

        match result {
            Ok(db_metadata_list) => {
                let mut metadata_list = Vec::new();
                for db_metadata in db_metadata_list {
                    match EigenAvsMetadata::try_from(db_metadata) {
                        Ok(metadata) => metadata_list.push(metadata),
                        Err(e) => {
                            error!("Failed to convert DB metadata: {}", e);
                            return Err(DatabaseError::FailedConversion(e.to_string()));
                        }
                    }
                }
                Ok(metadata_list)
            }
            Err(e) => {
                error!(
                    "Failed to get all metadata for metadata_uri: {}, error: {}",
                    metadata_uri, e
                );
                Err(DatabaseError::SqlxError(e))
            }
        }
    }

    pub async fn get_all_for_address(
        pool: &PgPool,
        address: Address,
    ) -> Result<Vec<EigenAvsMetadata>, DatabaseError> {
        debug!("Getting all metadata for address: {}", address);

        let address_str = format!("{:?}", address);

        let result = sqlx::query_as!(
            DbEigenAvsMetadata,
            r#"
            SELECT
                id, address, block_number, log_index, metadata_uri,
                name, description, website, logo, twitter, created_at
            FROM eigen_avs_metadata
            WHERE address = $1
            ORDER BY block_number DESC, log_index DESC
            "#,
            address_str,
        )
        .fetch_all(pool)
        .await;

        match result {
            Ok(db_metadata_list) => {
                let mut metadata_list = Vec::new();
                for db_metadata in db_metadata_list {
                    match EigenAvsMetadata::try_from(db_metadata) {
                        Ok(metadata) => metadata_list.push(metadata),
                        Err(e) => {
                            error!("Failed to convert DB metadata: {}", e);
                            return Err(DatabaseError::FailedConversion(e.to_string()));
                        }
                    }
                }
                Ok(metadata_list)
            }
            Err(e) => {
                error!("Failed to get all metadata for address: {}, error: {}", address, e);
                Err(DatabaseError::SqlxError(e))
            }
        }
    }

    pub async fn get_all_address_or_metadata_uri(
        pool: &PgPool,
        address: Address,
        metadata_uri: String,
    ) -> Result<Vec<EigenAvsMetadata>, DatabaseError> {
        debug!("Getting all metadata for address: {} at block: {}", address, metadata_uri);

        let address_str = format!("{:?}", address);

        let result = sqlx::query_as!(
            DbEigenAvsMetadata,
            r#"
            SELECT
                id, address, block_number, log_index, metadata_uri,
                name, description, website, logo, twitter, created_at
            FROM eigen_avs_metadata
            WHERE address = $1 OR metadata_uri = $2
            ORDER BY block_number DESC, log_index DESC
            "#,
            address_str,
            metadata_uri,
        )
        .fetch_all(pool)
        .await;

        match result {
            Ok(db_metadata_list) => {
                let mut metadata_list = Vec::new();
                for db_metadata in db_metadata_list {
                    match EigenAvsMetadata::try_from(db_metadata) {
                        Ok(metadata) => metadata_list.push(metadata),
                        Err(e) => {
                            error!("Failed to convert DB metadata: {}", e);
                            return Err(DatabaseError::FailedConversion(e.to_string()));
                        }
                    }
                }
                Ok(metadata_list)
            }
            Err(e) => {
                error!("Failed to get all metadata for address: {}, error: {}", address_str, e);
                Err(DatabaseError::SqlxError(e))
            }
        }
    }

    /// The point of this function is to see if an AVS has already been registered. If none of these
    /// things are the same, it could still be an AVS that was already registered, but they probably
    /// screwed something up.
    pub async fn search_for_avs(
        pool: &PgPool,
        address: Address,
        metadata_uri: String,
        name: String,
        website: String,
        twitter: String,
    ) -> Result<i64, DatabaseError> {
        debug!("Getting all metadata for address: {} at block: {}", address, metadata_uri);

        let address_str = format!("{:?}", address);

        let result = sqlx::query_scalar!(
            r#"
            SELECT
                COUNT(*)::BIGINT
            FROM eigen_avs_metadata
            WHERE address = $1 OR metadata_uri = $2 OR name = $3 OR website = $4 OR twitter = $5
            "#,
            address_str,
            metadata_uri,
            name,
            website,
            twitter,
        )
        .fetch_one(pool)
        .await;

        match result {
            Ok(count) => Ok(count.unwrap_or(0)),
            Err(e) => {
                error!(
                    "Failed to get count of metadata for address: {}, error: {}",
                    address_str, e
                );
                Err(DatabaseError::SqlxError(e))
            }
        }
    }

    /// Get metadata for a specific AVS address at a specific block
    pub async fn get_for_address_at_block(
        pool: &PgPool,
        address: Address,
        block_number: i64,
    ) -> Result<Option<EigenAvsMetadata>, DatabaseError> {
        debug!("Getting metadata for address: {} at block: {}", address, block_number);

        let address_str = format!("{:?}", address);

        let result = sqlx::query_as!(
            DbEigenAvsMetadata,
            r#"
            SELECT
                id, address, block_number, log_index, metadata_uri,
                name, description, website, logo, twitter, created_at
            FROM eigen_avs_metadata
            WHERE address = $1 AND block_number = $2
            ORDER BY log_index DESC
            "#,
            address_str,
            block_number,
        )
        .fetch_optional(pool)
        .await;

        match result {
            Ok(Some(db_metadata)) => {
                let metadata = EigenAvsMetadata::try_from(db_metadata)
                    .map_err(|e| DatabaseError::FailedConversion(e.to_string()))?;
                Ok(Some(metadata))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                error!(
                    "Failed to get metadata for address: {} at block: {}, error: {}",
                    address, block_number, e
                );
                Err(DatabaseError::SqlxError(e))
            }
        }
    }

    /// Delete metadata for a specific AVS address
    pub async fn delete_for_address(pool: &PgPool, address: Address) -> Result<(), DatabaseError> {
        debug!("Deleting all metadata for address: {}", address);

        let address_str = format!("{:?}", address);

        let result = sqlx::query!(
            r#"
            DELETE FROM eigen_avs_metadata
            WHERE address = $1
            "#,
            address_str,
        )
        .execute(pool)
        .await;

        match result {
            Ok(_) => {
                info!("Successfully deleted all metadata for address: {}", address);
                Ok(())
            }
            Err(e) => {
                error!("Failed to delete metadata for address: {}, error: {}", address, e);
                Err(DatabaseError::SqlxError(e))
            }
        }
    }

    /// Delete metadata for a specific AVS address at a specific block
    pub async fn delete_for_address_block_log(
        pool: &PgPool,
        address: Address,
        block_number: i64,
        log_index: i32,
    ) -> Result<(), DatabaseError> {
        debug!("Deleting metadata for address: {} at block: {}", address, block_number);

        let address_str = format!("{:?}", address);

        let result = sqlx::query!(
            r#"
            DELETE FROM eigen_avs_metadata
            WHERE address = $1 AND block_number = $2 AND log_index = $3
            "#,
            address_str,
            block_number,
            log_index,
        )
        .execute(pool)
        .await;

        match result {
            Ok(_) => {
                info!(
                    "Successfully deleted metadata for address: {} at block: {}",
                    address, block_number
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    "Failed to delete metadata for address: {} at block: {}, error: {}",
                    address, block_number, e
                );
                Err(DatabaseError::SqlxError(e))
            }
        }
    }
}
