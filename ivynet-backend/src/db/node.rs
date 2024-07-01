use crate::error::BackendError;

use chrono::{NaiveDateTime, Utc};
use ivynet_core::ethers::types::Address;
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};

use super::account::Account;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub node_id: Address,
    pub organization_id: i64,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Clone, Debug)]
pub struct DbNode {
    pub node_id: Vec<u8>,
    pub organization_id: i64,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl DbNode {
    pub async fn get_all_for_account(
        pool: &PgPool,
        account: &Account,
    ) -> Result<Vec<Node>, BackendError> {
        let nodes = sqlx::query_as!(
            DbNode,
            "SELECT node_id, organization_id, created_at, updated_at FROM node WHERE organization_id = $1",
            account.organization_id
        )
        .fetch_all(pool) // -> Vec<Country>
        .await?;

        Ok(nodes.into_iter().map(|n| n.into()).collect())
    }

    pub async fn get(pool: &PgPool, node_id: &Address) -> Result<Node, BackendError> {
        let node = sqlx::query_as!(
            DbNode,
            "SELECT node_id, organization_id, created_at, updated_at FROM node WHERE node_id = $1",
            node_id.as_bytes()
        )
        .fetch_one(pool)
        .await?;

        Ok(node.into())
    }

    pub async fn new(
        pool: &PgPool,
        account: &Account,
        node_id: &Address,
    ) -> Result<(), BackendError> {
        let now: NaiveDateTime = Utc::now().naive_utc();

        query!(
            "INSERT INTO node (node_id, organization_id, created_at, updated_at) values ($1, $2, $3, $4)",
            Some(node_id.as_bytes()),
            Some(account.user_id),
            Some(now),
            Some(now)
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &PgPool, node_id: &Address) -> Result<(), BackendError> {
        query!("DELETE FROM node WHERE node_id = $1", node_id.as_bytes())
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn purge(pool: &PgPool) -> Result<(), BackendError> {
        query!("DELETE FROM node").execute(pool).await?;
        Ok(())
    }
}

impl From<DbNode> for Node {
    fn from(value: DbNode) -> Self {
        Self {
            node_id: Address::from_slice(&value.node_id),
            organization_id: value.organization_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
