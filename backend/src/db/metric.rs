use std::collections::HashMap;

use crate::error::BackendError;

use chrono::{NaiveDateTime, Utc};
use ivynet_core::{ethers::types::Address, grpc::messages::Metrics};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{query, PgPool};
use utoipa::ToSchema;

use super::Node;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct Metric {
    pub node_id: Address,
    pub name: String,
    pub value: f64,
    pub attributes: Option<HashMap<String, String>>,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct DbMetric {
    pub node_id: Vec<u8>,
    pub name: String,
    pub value: f64,
    pub attributes: Option<sqlx::types::Json<HashMap<String, String>>>,
    pub created_at: Option<NaiveDateTime>,
}

impl From<DbMetric> for Metric {
    fn from(value: DbMetric) -> Self {
        Self {
            node_id: Address::from_slice(&value.node_id),
            name: value.name.clone(),
            value: value.value,
            attributes: value.attributes.as_ref().map(|v| v.0.clone()),
            created_at: value.created_at,
        }
    }
}

impl From<&Metrics> for Metric {
    fn from(value: &Metrics) -> Self {
        let mut attr_map = HashMap::new();
        for attr in &value.attributes {
            attr_map.insert(attr.name.clone(), attr.value.clone());
        }
        Self {
            node_id: Address::zero(),
            name: value.name.clone(),
            value: value.value,
            attributes: if !attr_map.is_empty() { Some(attr_map) } else { None },
            created_at: None,
        }
    }
}

impl Metric {
    pub async fn get_all_for_node(pool: &PgPool, node: &Node) -> Result<Vec<Metric>, BackendError> {
        let metrics = sqlx::query_as!(
            DbMetric,
            r#"SELECT node_id, name, value, attributes as "attributes: sqlx::types::Json<HashMap<String,String>>" , created_at FROM metric WHERE node_id = $1"#,
            node.node_id.as_bytes()
        )
        .fetch_all(pool) // -> Vec<Country>
        .await?;

        Ok(metrics.into_iter().map(|n| n.into()).collect())
    }

    pub async fn record(
        pool: &PgPool,
        node: &Node,
        metrics: &[Metric],
    ) -> Result<(), BackendError> {
        // Remove old metrics for the node first
        query!("DELETE FROM metric WHERE node_id = $1", node.node_id.as_bytes())
            .execute(pool)
            .await?;

        let now: NaiveDateTime = Utc::now().naive_utc();
        for metric in metrics {
            query!(
            "INSERT INTO metric (node_id, name, value, attributes, created_at) values ($1, $2, $3, $4, $5)",
            Some(node.node_id.as_bytes()),
            Some(&metric.name),
            Some(metric.value),
            metric.attributes.as_ref().map(|v| json!(v)),
            Some(now)
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }
}
