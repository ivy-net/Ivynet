use std::collections::HashMap;

use crate::error::BackendError;

use chrono::{NaiveDateTime, Utc};
use ivynet_core::grpc::messages::Metrics;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
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

    pub async fn get_machine_metrics_only(
        pool: &PgPool,
        machine_id: Uuid,
    ) -> Result<HashMap<String, Metric>, BackendError> {
        let metrics = sqlx::query_as!(
            DbMetric,
            r#"SELECT
                machine_id, avs_name, name, value,
                attributes as "attributes: sqlx::types::Json<HashMap<String,String>>",
                created_at
               FROM
                metric
               WHERE
                (machine_id = $1 AND avs_name IS NULL)"#,
            machine_id
        )
        .fetch_all(pool)
        .await?;

        Ok(metrics.into_iter().map(|m| (m.name.clone(), m.into())).collect())
    }

    pub async fn get_all_for_machine(
        pool: &PgPool,
        machine_id: Uuid,
    ) -> Result<HashMap<String, Metric>, BackendError> {
        let metrics = sqlx::query_as!(
            DbMetric,
            r#"SELECT
                machine_id, avs_name, name, value,
                attributes as "attributes: sqlx::types::Json<HashMap<String,String>>",
                created_at
               FROM
                metric
               WHERE
                machine_id = $1"#,
            machine_id
        )
        .fetch_all(pool)
        .await?;

        Ok(metrics.into_iter().map(|m| (m.name.clone(), m.into())).collect())
    }

    /// Returns a HashMap of metrics organized by metric name.
    pub async fn get_organized_for_avs(
        pool: &PgPool,
        machine_id: Uuid,
        avs_name: &str,
    ) -> Result<HashMap<String, Metric>, BackendError> {
        let metrics = Metric::get_all_for_avs(pool, machine_id, avs_name).await?;

        let mut organized = HashMap::new();

        for metric in metrics.into_iter() {
            organized.insert(metric.name.clone(), metric);
        }

        Ok(organized)
    }

    pub async fn record(
        pool: &PgPool,
        machine_id: Uuid,
        avs_name: Option<&str>,
        metrics: &[Metric],
    ) -> Result<(), BackendError> {
        let now: NaiveDateTime = Utc::now().naive_utc();
        let mut tx = pool.begin().await?;

        // Delete existing metrics (keeping this part the same)
        match avs_name {
            Some(name) => {
                sqlx::query!(
                    "DELETE FROM metric WHERE machine_id = $1 AND avs_name = $2",
                    machine_id,
                    name
                )
                .execute(&mut *tx)
                .await?;
            }
            None => {
                sqlx::query!(
                    "DELETE FROM metric WHERE machine_id = $1 AND avs_name IS NULL",
                    machine_id
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        // Start COPY operation
        let mut copy = tx
            .copy_in_raw(
                "COPY metric (machine_id, avs_name, name, value, attributes, created_at) FROM STDIN",
            )
            .await?;

        for metric in metrics {
            let attributes_str = match &metric.attributes {
                Some(attrs) => {
                    let attrs_string = serde_json::to_string(attrs)
                        .map_err(|e| BackendError::SerializationError(e.to_string()))?;
                    Self::escape_copy_value(&attrs_string)
                }
                _ => String::from("\\N"),
            };

            let avs_name_str =
                avs_name.map(Self::escape_copy_value).unwrap_or_else(|| String::from("\\N"));

            let row = format!(
                "{}\t{}\t{}\t{}\t{}\t{}\n",
                machine_id,
                avs_name_str,
                Self::escape_copy_value(&metric.name),
                metric.value,
                attributes_str,
                now
            );
            copy.send(row.as_bytes()).await?;
        }

        copy.finish().await?;
        tx.commit().await?;
        Ok(())
    }

    fn escape_copy_value(value: &str) -> String {
        if value.is_empty() {
            return "\\N".to_string();
        }

        value.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n").replace('\r', "\\r")
    }
}
