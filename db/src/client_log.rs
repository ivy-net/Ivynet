use crate::{error::DatabaseError, log::LogLevel};
use chrono::{DateTime, NaiveDateTime, Utc};
use ivynet_core::ethers::types::Address;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use sqlx::{query, query_as, PgPool};
use std::{collections::HashMap, fmt::Display, str::FromStr};
use utoipa::ToSchema;
use uuid::Uuid;

pub const DAYS_TO_KEEP_LOGS: i64 = 2;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow, PartialEq, Eq)]
pub struct ClientLog {
    pub client_id: Address,
    pub log: String,
    pub log_level: LogLevel,
    pub created_at: Option<i64>,
    #[serde(flatten, deserialize_with = "deserialize_other_fields")]
    pub other_fields: Option<HashMap<String, String>>,
}

impl Display for ClientLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ContainerLog {{ client_id: {}, log: {}, log_level: {:?}, created_at: {:?}, other_fields: {:?} }}",
            self.client_id, self.log, self.log_level, self.created_at, self.other_fields
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow, PartialEq, Eq)]
pub struct ClientDbLog {
    pub client_id: Vec<u8>,
    pub log: String,
    pub log_level: LogLevel,
    pub created_at: NaiveDateTime,
    pub other_fields: Option<sqlx::types::Json<HashMap<String, String>>>,
}

impl From<ClientDbLog> for ClientLog {
    fn from(value: ClientDbLog) -> Self {
        Self {
            client_id: Address::from_slice(value.client_id.as_slice()),
            log: value.log,
            log_level: value.log_level,
            created_at: Some(value.created_at.and_utc().timestamp()),
            other_fields: value.other_fields.as_ref().map(|v| v.0.clone()),
        }
    }
}

impl From<&ClientLog> for ClientDbLog {
    fn from(value: &ClientLog) -> Self {
        Self {
            client_id: value.client_id.as_bytes().to_vec(),
            log: value.log.clone(),
            log_level: value.log_level,
            created_at: DateTime::from_timestamp(value.created_at.expect("invalid "), 0)
                .expect("Invalid naive utc")
                .naive_utc(),
            other_fields: value.other_fields.as_ref().map(|v| sqlx::types::Json(v.clone())),
        }
    }
}
impl ClientLog {
    pub async fn record(
        pool: &PgPool,
        log: &ClientLog, // Accept a slice of logs for batch insertion
    ) -> Result<(), DatabaseError> {
        // cast timestamp to NaiveDateTime or get current time
        let created_at =
            DateTime::from_timestamp(log.created_at.unwrap_or_else(|| Utc::now().timestamp()), 0)
                .expect("Could not construct datetime")
                .naive_utc();

        query!(
            "INSERT INTO client_log (client_id, log, log_level, created_at, other_fields) VALUES ($1, $2, $3, $4, $5)",
            log.client_id.as_bytes().to_vec(),
            log.log,
            log.log_level as LogLevel,
            created_at,
            log.other_fields.as_ref().map(|v| json!(v)),
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_all_for_client(
        pool: &PgPool,
        client_id: Address,
    ) -> Result<Vec<ClientLog>, DatabaseError> {
        let db_logs: Vec<ClientDbLog> = query_as!(
            ClientDbLog,
            r#"SELECT client_id, log, log_level AS "log_level!: LogLevel", created_at, other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>" FROM client_log WHERE client_id = $1"#,
            client_id.as_bytes().to_vec()
        )
        .fetch_all(pool)
        .await?;

        let logs = db_logs.into_iter().map(ClientLog::from).collect::<Vec<_>>();
        Ok(logs)
    }

    pub async fn delete_old_logs(pool: &PgPool) -> Result<(), DatabaseError> {
        const BATCH_SIZE: i64 = 100;

        let days_ago = Utc::now().timestamp() - (DAYS_TO_KEEP_LOGS * 24 * 60 * 60);
        let cutoff_date =
            DateTime::from_timestamp(days_ago, 0).expect("Invalid timestamp").naive_utc();

        let mut total_deleted = 0;

        loop {
            let deleted_count = sqlx::query!(
                r#"
                DELETE FROM client_log
                WHERE created_at < $1
                AND ctid = ANY (
                    SELECT ctid
                    FROM log
                    WHERE created_at < $1
                    LIMIT $2
                )
                "#,
                cutoff_date,
                BATCH_SIZE
            )
            .execute(pool)
            .await?;

            let affected = deleted_count.rows_affected();
            total_deleted += affected;

            println!("Deleted batch of {} logs", affected);

            if affected == 0 || affected < BATCH_SIZE as u64 {
                break;
            }

            tokio::task::yield_now().await;
        }

        println!("Total deleted logs: {}", total_deleted);
        Ok(())
    }
}

// Deserialize other fields as a HashMap, any nested fields will be serialized as strings
fn deserialize_other_fields<'de, D>(
    deserializer: D,
) -> Result<Option<HashMap<String, String>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;

    let mut map = HashMap::new();

    if let Value::Object(obj) = value {
        for (key, val) in obj.into_iter() {
            let val_str = match val {
                Value::String(s) => s,
                _ => val.to_string(),
            };
            map.insert(key, val_str);
        }
    }
    if map.is_empty() {
        return Ok(None);
    }
    Ok(Some(map))
}

#[cfg(test)]
mod test_alerts_db {
    use sqlx::PgPool;
    use uuid::Uuid;

    use super::*;

    fn debug_address() -> Uuid {
        Address::from_slice(&[1; 20]).into()
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/alerts_active.sql",)
    )]
    #[ignore]
    async fn test_add_new_client_log(pool: PgPool) {
        let alerts_active = alerts_active::ActiveAlert::get_all(&pool).await.unwrap();

        let num_alerts = alerts_active.len();

        let new_alert = alerts_active::NewAlert {
            alert_type: AlertType::Custom,
            machine_id: Uuid::parse_str("dcbf22c7-9d96-47ac-bf06-62d6544e440d").unwrap(),
            node_name: "test".to_string(),
            created_at: chrono::Utc::now().naive_utc(),
        };
        let new_alert_uuid = new_alert.generate_uuid();

        alerts_active::ActiveAlert::insert_one(&pool, &new_alert).await.unwrap();

        let alerts_active = alerts_active::ActiveAlert::get_all(&pool).await.unwrap();

        assert_eq!(alerts_active.len(), num_alerts + 1);

        let new_db_alert =
            alerts_active::ActiveAlert::get(&pool, new_alert_uuid).await.unwrap().unwrap();

        assert_eq!(new_db_alert.alert_type, new_alert.alert_type);
        assert_eq!(new_db_alert.machine_id, new_alert.machine_id);
        assert_eq!(new_db_alert.node_name, new_alert.node_name);
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/alerts_active.sql",)
    )]
    #[ignore]
    async fn test_get_client_log_by_id(pool: PgPool) {}

