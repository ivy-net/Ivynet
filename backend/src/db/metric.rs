use std::collections::HashMap;

use crate::error::BackendError;

use chrono::{NaiveDateTime, Utc};
use ivynet_core::grpc::messages::Metrics;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{query, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct Metric {
    pub machine_id: Uuid,
    pub avs_name: Option<String>,
    pub name: String,
    pub value: f64,
    pub attributes: Option<HashMap<String, String>>,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct DbMetric {
    pub machine_id: Uuid,
    pub avs_name: Option<String>,
    pub name: String,
    pub value: f64,
    pub attributes: Option<sqlx::types::Json<HashMap<String, String>>>,
    pub created_at: Option<NaiveDateTime>,
}

impl From<DbMetric> for Metric {
    fn from(value: DbMetric) -> Self {
        Self {
            machine_id: value.machine_id,
            avs_name: value.avs_name.clone(),
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
            machine_id: Uuid::nil(),
            avs_name: None,
            name: value.name.clone(),
            value: value.value,
            attributes: if !attr_map.is_empty() { Some(attr_map) } else { None },
            created_at: None,
        }
    }
}

impl Metric {
    pub async fn get_all_for_avs(
        pool: &PgPool,
        machine_id: Uuid,
        avs_name: &str,
    ) -> Result<Vec<Metric>, BackendError> {
        let metrics = sqlx::query_as!(
            DbMetric,
            r#"SELECT
                machine_id, avs_name, name, value,
                attributes as "attributes: sqlx::types::Json<HashMap<String,String>>",
                created_at
               FROM
                metric
               WHERE
                machine_id = $1
                AND
                 (avs_name = $2 OR avs_name IS NULL)"#,
            machine_id,
            avs_name
        )
        .fetch_all(pool)
        .await?;

        Ok(metrics.into_iter().map(|n| n.into()).collect())
    }

    pub async fn get_organized_for_avs(
        pool: &PgPool,
        machine_id: Uuid,
        avs_name: &str,
    ) -> Result<HashMap<String, Metric>, BackendError> {
        let metrics = Metric::get_all_for_avs(pool, machine_id, avs_name).await?;

        let mut organized = HashMap::new();

        for metric in &metrics {
            organized.insert(metric.name.clone(), metric.clone());
        }

        Ok(organized)
    }

    pub async fn record(
        pool: &PgPool,
        machine_id: Uuid,
        avs_name: &str,
        metrics: &[Metric],
    ) -> Result<(), BackendError> {
        // Remove old metrics for the node first
        query!("DELETE FROM metric WHERE machine_id = $1 AND avs_name = $2", machine_id, avs_name)
            .execute(pool)
            .await?;

        let now: NaiveDateTime = Utc::now().naive_utc();
        for metric in metrics {
            query!(
            "INSERT INTO metric (machine_id, avs_name, name, value, attributes, created_at) values ($1, $2, $3, $4, $5, $6)",
            Some(machine_id),
            Some(avs_name),
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
