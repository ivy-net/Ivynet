use ivynet_core::{avs::names::AvsName, ethers::types::Address};
use semver::Version;
use serde::{Deserialize, Serialize};
use sqlx::query;
use utoipa::ToSchema;

use crate::error::BackendError;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct NodeData {
    pub serial_id: i32,
    pub node_id: Address,
    pub avs_name: AvsName,
    pub avs_version: Version,
    pub active_set: bool,
    pub operator_id: Option<Address>,
}

/// Database representation of NodeData
/// Chose to not use node_id as the primary key because
/// it needs to be easy to query multiple avs per node_id
/// for the future
#[derive(Clone, Debug)]
pub struct DbNodeData {
    pub id: i32,
    pub node_id: Vec<u8>,
    pub avs_name: String,
    pub avs_version: String,
    pub active_set: bool,
    pub operator_id: Option<Vec<u8>>,
}

impl From<DbNodeData> for NodeData {
    fn from(db_node_data: DbNodeData) -> Self {
        NodeData {
            serial_id: db_node_data.id,
            node_id: Address::from_slice(&db_node_data.node_id),
            avs_name: AvsName::from(db_node_data.avs_name.as_str()),
            avs_version: Version::parse(&db_node_data.avs_version)
                .expect("Cannot parse version on dbNodeData"),
            active_set: db_node_data.active_set,
            operator_id: {
                db_node_data.operator_id.map(|operator_id| Address::from_slice(&operator_id))
            },
        }
    }
}

impl DbNodeData {
    pub async fn get_all_node_data(
        pool: &sqlx::PgPool,
        node_id: &Address,
    ) -> Result<Vec<NodeData>, BackendError> {
        let nodes_data: Vec<DbNodeData> = sqlx::query_as!(
            DbNodeData,
            "SELECT id, node_id, avs_name, avs_version, active_set, operator_id FROM node_data WHERE node_id = $1",
            node_id.as_bytes()
        )
        .fetch_all(pool)
        .await?;

        let node_data: Vec<NodeData> = nodes_data.into_iter().map(NodeData::from).collect();
        Ok(node_data)
    }

    // This could still return multiple values if they have multiple operators
    // each running the same avs
    pub async fn get_avs_node_data(
        pool: &sqlx::PgPool,
        node_id: &Address,
        avs_name: &AvsName,
    ) -> Result<Vec<NodeData>, BackendError> {
        let nodes_data: Vec<DbNodeData> = sqlx::query_as!(
            DbNodeData,
            "SELECT id, node_id, avs_name, avs_version, active_set, operator_id FROM node_data WHERE node_id = $1 AND avs_name = $2",
            node_id.as_bytes(),
            avs_name.clone().to_string()
        )
        .fetch_all(pool)
        .await?;

        let node_data: Vec<NodeData> = nodes_data.into_iter().map(NodeData::from).collect();
        Ok(node_data)
    }

    pub async fn get_operator_node_data(
        pool: &sqlx::PgPool,
        operator_id: &Address,
    ) -> Result<Vec<NodeData>, BackendError> {
        let nodes_data: Vec<DbNodeData> = sqlx::query_as!(
            DbNodeData,
            "SELECT id, node_id, avs_name, avs_version, active_set, operator_id FROM node_data WHERE operator_id = $1",
            operator_id.as_bytes()
        )
        .fetch_all(pool)
        .await?;

        let node_data: Vec<NodeData> = nodes_data.into_iter().map(NodeData::from).collect();
        Ok(node_data)
    }

    pub async fn record_avs_node_data(
        pool: &sqlx::PgPool,

        operator_id: &Address,
        node_id: &Address,
        avs_name: &AvsName,
        avs_version: &Version,
        active_set: bool,
    ) -> Result<(), BackendError> {
        query!(
            "INSERT INTO node_data (node_id, avs_name, avs_version, active_set, operator_id) values ($1, $2, $3, $4, $5)
            ON CONFLICT (operator_id, avs_name)
            DO UPDATE SET avs_version = $3, active_set = $4",
            Some(node_id.as_bytes()),
            avs_name.clone().to_string(),
            avs_version.to_string(),
            active_set,
            operator_id.as_bytes(),
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn set_active_set(
        pool: &sqlx::PgPool,
        operator_id: &Address,
        avs_name: &AvsName,
        active_set: bool,
    ) -> Result<(), BackendError> {
        query!(
            "UPDATE node_data SET active_set = $1 WHERE operator_id = $2 AND avs_name = $3",
            active_set,
            operator_id.as_bytes(),
            avs_name.clone().to_string()
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn set_avs_version(
        pool: &sqlx::PgPool,
        operator_id: &Address,
        avs_name: &AvsName,
        avs_version: &Version,
    ) -> Result<(), BackendError> {
        query!(
            "UPDATE node_data SET avs_version = $1 WHERE operator_id = $2 AND avs_name = $3",
            avs_version.to_string(),
            operator_id.as_bytes(),
            avs_name.clone().to_string()
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete_avs_operator_data(
        pool: &sqlx::PgPool,
        operator_id: &Address,
        avs_name: &AvsName,
    ) -> Result<(), BackendError> {
        query!(
            "DELETE FROM node_data WHERE operator_id = $1 AND avs_name = $2",
            operator_id.as_bytes(),
            avs_name.clone().to_string()
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete_all_node_data(
        pool: &sqlx::PgPool,
        node_id: &Address,
    ) -> Result<(), BackendError> {
        query!("DELETE FROM node_data WHERE node_id = $1", node_id.as_bytes())
            .execute(pool)
            .await?;
        Ok(())
    }
}
