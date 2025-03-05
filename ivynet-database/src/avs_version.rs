use std::collections::HashMap;

use super::utils::try_parse_chain;
use chrono::NaiveDateTime;
use ethers::types::Chain;
use ivynet_node_type::NodeType;
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};
use utoipa::ToSchema;

use crate::error::DatabaseError;

/// Represents version information for an AVS node
#[derive(Clone, Serialize, Deserialize, ToSchema, Eq, PartialEq, Debug)]
pub struct AvsVersionData {
    #[serde(flatten)]
    pub id: NodeTypeId,
    #[serde(flatten)]
    pub vd: VersionData,
}

/// Unique identifier for a node type and chain combination
/// NOTE: When this is serialized, the NodeType is serialized as a string which differs from the
/// to_string() implementation.
#[derive(Clone, Serialize, Deserialize, ToSchema, Eq, PartialEq, Hash, Debug)]
pub struct NodeTypeId {
    pub node_type: NodeType,
    pub chain: Chain,
}

/// Represents version information for an AVS node
#[derive(Clone, Serialize, Deserialize, ToSchema, Eq, PartialEq, Debug)]
pub struct VersionData {
    pub latest_version: String,
    // Necessary for fixed-version node types (`latest`, `holesky`, etc.)
    pub latest_version_digest: String,
    pub breaking_change_version: Option<String>,
    pub breaking_change_datetime: Option<NaiveDateTime>,
}

#[derive(Clone, sqlx::FromRow, Debug)]
pub struct DbAvsVersionData {
    pub id: i32,
    pub node_type: String,
    pub chain: String,
    pub latest_version_tag: String,
    pub latest_version_digest: String,
    pub breaking_change_tag: Option<String>,
    pub breaking_change_datetime: Option<NaiveDateTime>,
}

impl TryFrom<DbAvsVersionData> for AvsVersionData {
    type Error = DatabaseError;

    fn try_from(db: DbAvsVersionData) -> Result<Self, Self::Error> {
        let version_data = VersionData {
            latest_version: db.latest_version_tag,
            latest_version_digest: db.latest_version_digest,
            breaking_change_version: db.breaking_change_tag.to_owned(),
            breaking_change_datetime: db.breaking_change_datetime,
        };

        Ok(Self {
            id: NodeTypeId {
                node_type: NodeType::from(db.node_type.as_str()),
                chain: try_parse_chain(&db.chain).map_err(|_| DatabaseError::InvalidChain)?,
            },
            vd: version_data,
        })
    }
}

impl DbAvsVersionData {
    /// Retrieves all AVS version data from the database
    pub async fn get_all_avs_version(
        pool: &PgPool,
    ) -> Result<HashMap<NodeTypeId, VersionData>, DatabaseError> {
        let versions =
            sqlx::query_as!(Self, "SELECT * FROM avs_version_data").fetch_all(pool).await?;

        Ok(versions
            .into_iter()
            .filter_map(|data| AvsVersionData::try_from(data).ok().map(|data| (data.id, data.vd)))
            .collect())
    }

    /// Retrieves AVS version data for a specific node type
    pub async fn get_avs_version(
        pool: &PgPool,
        node_type: &NodeType,
    ) -> Result<Vec<AvsVersionData>, DatabaseError> {
        let versions = sqlx::query_as!(
            Self,
            "SELECT * FROM avs_version_data WHERE node_type = $1",
            node_type.to_string()
        )
        .fetch_all(pool)
        .await?;

        Ok(versions.into_iter().filter_map(|data| AvsVersionData::try_from(data).ok()).collect())
    }

    /// Retrieves AVS version data for a specific node type and chain
    pub async fn get_avs_version_with_chain(
        pool: &PgPool,
        node_type: &NodeType,
        chain: &Chain,
    ) -> Result<Option<AvsVersionData>, DatabaseError> {
        let version = sqlx::query_as!(
            Self,
            "SELECT * FROM avs_version_data WHERE node_type = $1 AND chain = $2",
            node_type.to_string(),
            chain.to_string(),
        )
        .fetch_optional(pool)
        .await?;

        version.map(AvsVersionData::try_from).transpose()
    }

    /// Inserts new AVS version data
    pub async fn insert_avs_version(
        pool: &PgPool,
        data: &AvsVersionData,
    ) -> Result<(), DatabaseError> {
        let query = match (&data.vd.breaking_change_version, &data.vd.breaking_change_datetime) {
            (Some(ver), Some(dt)) => query!(
                "INSERT INTO avs_version_data (node_type, latest_version_tag, chain, breaking_change_tag, breaking_change_datetime)
                VALUES ($1, $2, $3, $4, $5)",
                data.id.node_type.to_string(),
                data.vd.latest_version,
                data.id.chain.to_string(),
                ver,
                dt,
            ),
            _ => query!(
                "INSERT INTO avs_version_data (node_type, latest_version_tag, chain)
                VALUES ($1, $2, $3)",
                data.id.node_type.to_string(),
                data.vd.latest_version,
                data.id.chain.to_string(),
            ),
        };

        query.execute(pool).await?;
        Ok(())
    }

    pub async fn delete_avs_version_data(
        pool: &sqlx::PgPool,
        node_type: &NodeType,
        chain: &Chain,
    ) -> Result<(), DatabaseError> {
        query!(
            "DELETE FROM avs_version_data WHERE node_type = $1 AND chain = $2",
            node_type.to_string(),
            chain.to_string(),
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn set_avs_version(
        pool: &sqlx::PgPool,
        node_type: &NodeType,
        chain: &Chain,
        latest_version_tag: &str,
        latest_version_digest: &str,
    ) -> Result<(), DatabaseError> {
        query!(
            "INSERT INTO avs_version_data (node_type, latest_version_tag, latest_version_digest, chain)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (node_type, chain) DO UPDATE SET latest_version_tag = $2, latest_version_digest = $3",
            node_type.to_string(),
            latest_version_tag,
            latest_version_digest,
            chain.to_string(),
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn set_breaking_change_version(
        pool: &sqlx::PgPool,
        node_type: &NodeType,
        chain: &Chain,
        breaking_change_version: &str,
        breaking_change_datetime: &NaiveDateTime,
    ) -> Result<(), DatabaseError> {
        query!(
            "UPDATE avs_version_data
            SET breaking_change_tag = $3, breaking_change_datetime = $4
            WHERE node_type = $1 AND chain = $2",
            node_type.to_string(),
            chain.to_string(),
            Some(breaking_change_version.to_string()),
            Some(breaking_change_datetime)
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete_avses_from_avs_version_data(
        pool: &sqlx::PgPool,
        avs_types_to_keep: &[NodeType],
    ) -> Result<(), DatabaseError> {
        // First get all unique node types from DB
        let all_types =
            sqlx::query!("SELECT DISTINCT node_type FROM avs_version_data").fetch_all(pool).await?;

        // Convert keep list to strings for comparison
        let keep_set: Vec<String> = avs_types_to_keep.iter().map(|t| t.to_string()).collect();

        // Find types to delete
        let types_to_delete: Vec<String> = all_types
            .into_iter()
            .map(|row| row.node_type)
            .filter(|node_type| !keep_set.contains(node_type))
            .collect();

        // If we found types to delete, remove them
        if !types_to_delete.is_empty() {
            let result = sqlx::query!(
                "DELETE FROM avs_version_data WHERE node_type = ANY($1)",
                &types_to_delete
            )
            .execute(pool)
            .await?;

            println!(
                "Deleted {} rows for node types: {:?}",
                result.rows_affected(),
                types_to_delete
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_avses_from_table() {
        let distinct_types = [
            "k-3-labs-avs",
            "automata",
            "k-3-labs-avs-holesky",
            "altlayer(altlayer-mach)",
            "aethos-holesky",
            "lagrange-zk-worker-mainnet",
            "altlayer(gm-network-mach)",
            "hyperlane",
            "lagrange-zk-worker-holesky",
            "open-layer-mainnet",
            "k3-labs-avs",
            "cyber-mach",
            "open-layer-holesky",
            "chainbase-network",
            "e-oracle",
            "eigen-da",
            "generic-altlayer-mach",
            "arpa-network-node-client",
            "gm-network-mach",
            "lagrange-state-committee",
            "predicate-operator",
            "eigenda",
            "dodo-chain-mach",
            "chainbase-network-v-2",
            "generic-altlayer",
            "altlayer(unknown)",
            "altlayer-mach(unknown)",
            "witness-chain",
            "eoracle",
            "altlayer-mach(dodo-chain)",
            "altlayer-mach",
            "altlayer-mach(xterio)",
            "chainbase-network-v-1",
            "predicate",
            "altlayer-mach(cyber)",
            "ava-protocol",
            "xterio-mach",
            "omni",
            "witnesschain",
        ];

        let node_types_to_keep: Vec<String> =
            NodeType::all_known_with_repo().iter().map(|t| t.to_string()).collect();

        let types_to_delete: Vec<&str> = distinct_types
            .into_iter()
            .filter(|node_type| !node_types_to_keep.iter().any(|keep| keep == node_type))
            .collect();

        let types_to_delete_contain: Vec<&str> = distinct_types
            .into_iter()
            .filter(|node_type| !node_types_to_keep.contains(&node_type.to_string()))
            .collect();

        println!("Types to delete: {:#?}", types_to_delete);
        println!("Types to delete contain: {:#?}", types_to_delete_contain);

        assert!(types_to_delete.len() == types_to_delete_contain.len());
    }
}
