use crate::{
    error::DatabaseError,
    log::{ContainerLog, LogLevel},
};
use chrono::{DateTime, NaiveDateTime, Utc};
use ethers::types::Address;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use sqlx::{query, query_as, PgPool};
use std::{collections::HashMap, fmt::Display};
use utoipa::ToSchema;

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

    /// Compatability function to record a clientlog from a containerlog formatted message. Derives
    /// the client_id from the machine_id present in the log in the psql call.
    pub async fn record_from_containerlog(
        pool: &PgPool,
        log: &ContainerLog,
    ) -> Result<(), DatabaseError> {
        let created_at =
            DateTime::from_timestamp(log.created_at.unwrap_or_else(|| Utc::now().timestamp()), 0)
                .expect("Could not construct datetime")
                .naive_utc();

        query!(
            "INSERT INTO client_log (client_id, log, log_level, created_at) VALUES ((SELECT client_id FROM machine WHERE machine_id = $1), $2, $3, $4)",
            log.machine_id,
            log.log,
            log.log_level as LogLevel,
            created_at,
            ).execute(pool).await?;
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

    pub async fn get_all_for_client_with_filters(
        pool: &PgPool,
        client_id: Address,
        from: Option<i64>,
        to: Option<i64>,
        log_level: Option<LogLevel>,
    ) -> Result<Vec<ClientLog>, DatabaseError> {
        let db_logs = match (from, to, log_level) {
            (Some(from), Some(to), Some(level)) => {
                let from_dt =
                    DateTime::from_timestamp(from, 0).map(|dt| dt.naive_utc()).ok_or_else(
                        || DatabaseError::InvalidInput("Invalid 'from' timestamp".into()),
                    )?;
                let to_dt = DateTime::from_timestamp(to, 0)
                    .map(|dt| dt.naive_utc())
                    .ok_or_else(|| DatabaseError::InvalidInput("Invalid 'to' timestamp".into()))?;

                query_as!(
                    ClientDbLog,
                    r#"SELECT client_id, log, log_level AS "log_level!: LogLevel", created_at,
                       other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>"
                       FROM client_log
                       WHERE client_id = $1
                       AND created_at >= $2
                       AND created_at <= $3
                       AND log_level = $4
                       ORDER BY created_at"#,
                    client_id.as_bytes().to_vec(),
                    from_dt,
                    to_dt,
                    level as LogLevel,
                )
                .fetch_all(pool)
                .await?
            }
            (Some(from), Some(to), None) => {
                let from_dt =
                    DateTime::from_timestamp(from, 0).map(|dt| dt.naive_utc()).ok_or_else(
                        || DatabaseError::InvalidInput("Invalid 'from' timestamp".into()),
                    )?;
                let to_dt = DateTime::from_timestamp(to, 0)
                    .map(|dt| dt.naive_utc())
                    .ok_or_else(|| DatabaseError::InvalidInput("Invalid 'to' timestamp".into()))?;

                query_as!(
                    ClientDbLog,
                    r#"SELECT client_id, log, log_level AS "log_level!: LogLevel", created_at,
                       other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>"
                       FROM client_log
                       WHERE client_id = $1
                       AND created_at >= $2
                       AND created_at <= $3
                       ORDER BY created_at"#,
                    client_id.as_bytes().to_vec(),
                    from_dt,
                    to_dt,
                )
                .fetch_all(pool)
                .await?
            }
            (None, None, Some(level)) => {
                query_as!(
                    ClientDbLog,
                    r#"SELECT client_id, log, log_level AS "log_level!: LogLevel", created_at,
                       other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>"
                       FROM client_log
                       WHERE client_id = $1
                       AND log_level = $2
                       ORDER BY created_at"#,
                    client_id.as_bytes().to_vec(),
                    level as LogLevel,
                )
                .fetch_all(pool)
                .await?
            }
            _ => {
                query_as!(
                    ClientDbLog,
                    r#"SELECT client_id, log, log_level AS "log_level!: LogLevel", created_at,
                       other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>"
                       FROM client_log
                       WHERE client_id = $1
                       ORDER BY created_at"#,
                    client_id.as_bytes().to_vec(),
                )
                .fetch_all(pool)
                .await?
            }
        };

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
mod test_client_logs_db {
    use sqlx::PgPool;

    use super::*;

    fn debug_address() -> Address {
        Address::from_slice(&[1; 20])
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../fixtures/new_user_registration.sql", "../fixtures/new_client_log.sql",)
    )]
    #[ignore]
    async fn test_add_new_client_log(pool: PgPool) {
        let logs = ClientLog::get_all_for_client(&pool, debug_address()).await.unwrap();

        let num_logs = logs.len();

        let new_log = ClientLog {
            client_id: debug_address(),
            log: "test".to_string(),
            log_level: LogLevel::Info,
            created_at: Some(Utc::now().timestamp()),
            other_fields: None,
        };

        ClientLog::record(&pool, &new_log).await.unwrap();

        let logs = ClientLog::get_all_for_client(&pool, debug_address()).await.unwrap();

        assert_eq!(logs.len(), num_logs + 1);

        let new_db_log = logs.iter().find(|l| l.log == new_log.log).unwrap();

        assert_eq!(new_db_log.log, new_log.log);
        assert_eq!(new_db_log.log_level, new_log.log_level);
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../fixtures/new_user_registration.sql", "../fixtures/new_client_log.sql",)
    )]
    #[ignore]
    async fn test_get_client_log_by_id(pool: PgPool) {
        let logs = ClientLog::get_all_for_client(&pool, debug_address()).await.unwrap();
        assert!(!logs.is_empty());
    }
}
