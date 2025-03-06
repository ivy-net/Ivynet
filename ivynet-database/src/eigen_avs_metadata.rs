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
    pub metadata_uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub website: Option<String>,
    pub logo: Option<String>,
    pub twitter: Option<String>,
    pub created_at: Option<NaiveDateTime>,
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
        metadata_uri: String,
    ) -> Result<(), DatabaseError> {
        debug!("Inserting AVS metadata for address: {}, block: {}", address, block_number);

        let address_str = format!("{:?}", address);

        let result = sqlx::query!(
            r#"
            INSERT INTO eigen_avs_metadata (
                address, block_number, metadata_uri, created_at
            ) VALUES (
                $1, $2, $3, NOW()
            )
            ON CONFLICT (address, block_number) DO UPDATE SET
                metadata_uri = EXCLUDED.metadata_uri
            "#,
            address_str,
            block_number,
            metadata_uri,
        )
        .execute(pool)
        .await;

        match result {
            Ok(_) => {
                info!(
                    "Successfully inserted AVS metadata for address: {}, block: {}",
                    address, block_number
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    "Failed to insert AVS metadata for address: {}, block: {}, error: {}",
                    address, block_number, e
                );
                Err(DatabaseError::SqlxError(e))
            }
        }
    }

    /// Update metadata content fields after fetching from URI
    pub async fn update_metadata_content(
        pool: &PgPool,
        address: Address,
        block_number: i64,
        name: Option<&str>,
        description: Option<&str>,
        website: Option<&str>,
        logo: Option<&str>,
        twitter: Option<&str>,
    ) -> Result<(), DatabaseError> {
        debug!("Updating metadata content for address: {}, block: {}", address, block_number);

        let address_str = format!("{:?}", address);

        let result = sqlx::query!(
            r#"
            UPDATE eigen_avs_metadata
            SET 
                name = $3,
                description = $4,
                website = $5,
                logo = $6,
                twitter = $7
            WHERE address = $1 AND block_number = $2
            "#,
            address_str,
            block_number,
            name,
            description,
            website,
            logo,
            twitter,
        )
        .execute(pool)
        .await;

        match result {
            Ok(_) => {
                info!(
                    "Successfully updated metadata content for address: {}, block: {}",
                    address, block_number
                );
                Ok(())
            }
            Err(e) => {
                error!(
                    "Failed to update metadata content for address: {}, block: {}, error: {}",
                    address, block_number, e
                );
                Err(DatabaseError::SqlxError(e))
            }
        }
    }

    /// Get the latest metadata for a specific AVS address
    pub async fn get_latest_for_address(
        pool: &PgPool,
        address: Address,
    ) -> Result<Option<EigenAvsMetadata>, DatabaseError> {
        debug!("Getting latest metadata for address: {}", address);

        let address_str = format!("{:?}", address);

        let result = sqlx::query_as!(
            DbEigenAvsMetadata,
            r#"
            SELECT 
                id, address, block_number, metadata_uri, 
                name, description, website, logo, twitter, created_at
            FROM eigen_avs_metadata
            WHERE address = $1
            ORDER BY block_number DESC
            LIMIT 1
            "#,
            address_str,
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
                error!("Failed to get latest metadata for address: {}, error: {}", address, e);
                Err(DatabaseError::SqlxError(e))
            }
        }
    }

    /// Get all metadata entries for a specific AVS address
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
                id, address, block_number, metadata_uri, 
                name, description, website, logo, twitter, created_at
            FROM eigen_avs_metadata
            WHERE address = $1
            ORDER BY block_number DESC
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
                id, address, block_number, metadata_uri, 
                name, description, website, logo, twitter, created_at
            FROM eigen_avs_metadata
            WHERE address = $1 AND block_number = $2
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
    pub async fn delete_for_address_at_block(
        pool: &PgPool,
        address: Address,
        block_number: i64,
    ) -> Result<(), DatabaseError> {
        debug!("Deleting metadata for address: {} at block: {}", address, block_number);

        let address_str = format!("{:?}", address);

        let result = sqlx::query!(
            r#"
            DELETE FROM eigen_avs_metadata
            WHERE address = $1 AND block_number = $2
            "#,
            address_str,
            block_number,
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
