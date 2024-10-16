use crate::error::BackendError;
use chrono::{DateTime, NaiveDateTime, Utc};
use ivynet_core::ethers::types::{Address, H160};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use sqlx::{query, query_as, PgPool};
use std::{collections::HashMap, str::FromStr};
use utoipa::ToSchema;

#[derive(
    Copy, Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Deserialize, Serialize, ToSchema,
)]
#[sqlx(type_name = "log_level", rename_all = "lowercase")]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Unknown,
}

impl FromStr for LogLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warning" => Ok(LogLevel::Warning),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow, PartialEq)]
pub struct ContainerLog {
    pub node_id: Option<Address>,
    /// Container ID. This is the ID of the container that generated the log. Optional as
    /// fluentd itself does not report its own container ID.
    pub container_id: Option<String>,
    pub container_name: String,
    pub log: String,
    pub log_level: LogLevel,
    pub created_at: Option<i64>,
    #[serde(flatten, deserialize_with = "deserialize_other_fields")]
    pub other_fields: Option<HashMap<String, String>>,
}

pub struct ContainerDbLog {
    pub node_id: Vec<u8>,
    pub container_id: Option<String>,
    pub container_name: String,
    pub log: String,
    pub log_level: LogLevel,
    pub created_at: NaiveDateTime,
    pub other_fields: Option<sqlx::types::Json<HashMap<String, String>>>,
}

impl From<ContainerDbLog> for ContainerLog {
    fn from(value: ContainerDbLog) -> Self {
        Self {
            node_id: Some(Address::from_slice(&value.node_id)),
            container_id: value.container_id,
            container_name: value.container_name,
            log: value.log,
            log_level: value.log_level,
            created_at: Some(value.created_at.and_utc().timestamp()),
            other_fields: value.other_fields.as_ref().map(|v| v.0.clone()),
        }
    }
}

impl From<&ContainerLog> for ContainerDbLog {
    fn from(value: &ContainerLog) -> Self {
        Self {
            node_id: value.node_id.as_ref().map(|v| v.as_bytes().to_vec()).unwrap_or_default(),
            container_id: value.container_id.clone(),
            container_name: value.container_name.clone(),
            log: value.log.clone(),
            log_level: value.log_level,
            created_at: DateTime::from_timestamp(value.created_at.expect("invalid "), 0)
                .expect("Invalid naive utc")
                .naive_utc(),
            other_fields: value.other_fields.as_ref().map(|v| sqlx::types::Json(v.clone())),
        }
    }
}

impl ContainerLog {
    pub async fn record(
        pool: &PgPool,
        log: &ContainerLog, // Accept a slice of logs for batch insertion
    ) -> Result<(), BackendError> {
        // cast timestamp to NaiveDateTime or get current time
        let created_at =
            DateTime::from_timestamp(log.created_at.unwrap_or_else(|| Utc::now().timestamp()), 0)
                .expect("Could not construct datetime")
                .naive_utc();
        query!(
            "INSERT INTO log (node_id, container_id, container_name, log, log_level, created_at, other_fields) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            log.node_id.as_ref().map(|v| v.as_bytes()),
            log.container_id,
            log.container_name,
            log.log,
            log.log_level as LogLevel,
            created_at,
            log.other_fields.as_ref().map(|v| json!(v)),
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_all_for_node(
        pool: &PgPool,
        node_id: H160,
    ) -> Result<Vec<ContainerLog>, BackendError> {
        let db_logs: Vec<ContainerDbLog> = query_as!(
            ContainerDbLog,
            r#"SELECT node_id, container_id, container_name, log, log_level AS "log_level!: LogLevel", created_at, other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>" FROM log WHERE node_id = $1"#,
            node_id.as_bytes()
        )
        .fetch_all(pool)
        .await?;

        let logs = db_logs.into_iter().map(ContainerLog::from).collect();
        Ok(logs)
    }

    pub async fn get_all_for_node_between_timestamps(
        pool: &PgPool,
        node_id: Address,
        start_timestamp: i64,
        end_timestamp: i64,
    ) -> Result<Vec<ContainerLog>, BackendError> {
        let db_logs: Vec<ContainerDbLog> = query_as!(
            ContainerDbLog,
            r#"SELECT node_id, container_id, container_name, log, log_level AS "log_level!: LogLevel", created_at, other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>" FROM log WHERE node_id = $1 AND created_at >= $2 AND created_at <= $3"#,
            node_id.as_bytes(),
            DateTime::from_timestamp(start_timestamp, 0).expect("Invalid timestamp").naive_utc(),
            DateTime::from_timestamp(end_timestamp, 0).expect("Invalid timestamp").naive_utc(),
        )
        .fetch_all(pool)
        .await?;

        let logs = db_logs.into_iter().map(ContainerLog::from).collect();
        Ok(logs)
    }

    pub async fn get_all_for_node_with_log_level(
        pool: &PgPool,
        node_id: H160,
        log_level: LogLevel,
    ) -> Result<Vec<ContainerLog>, BackendError> {
        let db_logs: Vec<ContainerDbLog> = query_as!(
            ContainerDbLog,
            r#"SELECT node_id, container_id, container_name, log, log_level AS "log_level!: LogLevel", created_at, other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>" FROM log WHERE node_id = $1 AND log_level = $2"#,
            node_id.as_bytes(),
            log_level as LogLevel,
        )
        .fetch_all(pool)
        .await?;

        let logs = db_logs.into_iter().map(ContainerLog::from).collect();
        Ok(logs)
    }

    pub async fn get_all_for_node_between_timestamps_with_log_level(
        pool: &PgPool,
        node_id: H160,
        from: i64,
        to: i64,
        log_level: LogLevel,
    ) -> Result<Vec<ContainerLog>, BackendError> {
        let db_logs: Vec<ContainerDbLog> = query_as!(
            ContainerDbLog,
            r#"SELECT node_id, container_id, container_name, log, log_level AS "log_level!: LogLevel", created_at, other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>" FROM log WHERE node_id = $1 AND created_at >= $2 AND created_at <= $3 AND log_level = $4"#,
            node_id.as_bytes(),
            DateTime::from_timestamp(from, 0).expect("Invalid timestamp").naive_utc(),
            DateTime::from_timestamp(to, 0).expect("Invalid timestamp").naive_utc(),
            log_level as LogLevel,
        )
        .fetch_all(pool)
        .await?;

        let logs = db_logs.into_iter().map(ContainerLog::from).collect();
        Ok(logs)
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
mod tests {
    use crate::db::log::ContainerLog;

    #[test]
    fn test_deserialize_fluentd_msg() {
        let log_str = "[{\"container_name\":\"fluentd\",\"created_at\":1729036593,\"log\":\"starting fluentd worker pid=16 ppid=7 worker=0\",\"log_level\":\"UNKNOWN\",\"pid\":16,\"ppid\":7,\"worker\":0},{\"bind\":\"0.0.0.0\",\"container_name\":\"fluentd\",\"created_at\":1729036593,\"log\":\"listening port port=24224 bind=\\\"0.0.0.0\\\"\",\"log_level\":\"UNKNOWN\",\"port\":24224},{\"container_name\":\"fluentd\",\"created_at\":1729036593,\"log\":\"fluentd worker is now running worker=0\",\"log_level\":\"UNKNOWN\",\"worker\":0}]";
        let container_logs = serde_json::from_str::<Vec<ContainerLog>>(log_str);
        assert!(container_logs.is_ok());

        let log_str = "[{\"container_id\":\"99b899e97e76cb3978f5b14627e0448515b33c4b17864348cbfa0f124ab35249\",\"container_name\":\"/eigenda-native-node\",\"created_at\":1729047253,\"log\":\"\\u001b[2mOct 16 02:54:13.038\\u001b[0m DBG \\u001b[2mnode/node.go:684\\u001b[0m Calling reachability check \\u001b[2mcomponent=\\u001b[0mNode \\u001b[2murl=\\u001b[0m\\\"https://dataapi-holesky.eigenda.xyz/api/v1/operators-info/port-check?operator_id=b8803017a8a79caf923721c33653df7a2153f127af95ecd72cc9fc064ff6afa0\\\"\",\"log_level\":\"DEBUG\",\"source\":\"stdout\"},{\"container_id\":\"99b899e97e76cb3978f5b14627e0448515b33c4b17864348cbfa0f124ab35249\",\"container_name\":\"/eigenda-native-node\",\"created_at\":1729047253,\"log\":\"\\u001b[2mOct 16 02:54:13.438\\u001b[0m \\u001b[93mWRN\\u001b[0m \\u001b[2mnode/node.go:695\\u001b[0m Reachability check operator id not found \\u001b[2mcomponent=\\u001b[0mNode \\u001b[2mstatus=\\u001b[0m404 \\u001b[2moperator_id=\\u001b[0mb8803017a8a79caf923721c33653df7a2153f127af95ecd72cc9fc064ff6afa0\",\"log_level\":\"WARNING\",\"source\":\"stdout\"}]";
        let container_logs = serde_json::from_str::<Vec<ContainerLog>>(log_str);
        let value = serde_json::from_str::<Vec<ContainerLog>>(log_str);
        println!("{:#?}", value);
        //         assert!(container_logs.is_ok());
    }
}

#[cfg(feature = "db_tests")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::node::DbNode;
    use chrono::Utc;
    use ivynet_core::ethers::types::Address;

    #[sqlx::test(fixtures(
        "../../../backend/fixtures/organization.sql",
        "../../../backend/fixtures/node.sql"
    ))]
    async fn test_insert_and_retrieve_logs(pool: PgPool) -> sqlx::Result<()> {
        let node_id = "0x00000000000000000000000000000000deadbeef".parse::<Address>().unwrap();

        let node = DbNode::get(&pool, &node_id).await;

        let log1 = ContainerLog {
            node_id: Some(node_id),
            container_id: Some("container1".to_string()),
            container_name: "test_container_1".to_string(),
            log: "This is a test log for container 1".to_string(),
            log_level: LogLevel::Info, // Assuming you have a LogLevel enum
            created_at: Some(Utc::now().timestamp()),
            other_fields: Some(HashMap::new()),
        };

        // wait for two seconds
        std::thread::sleep(std::time::Duration::from_secs(2));

        let log2 = ContainerLog {
            node_id: Some(node_id),
            container_id: Some("container2".to_string()),
            container_name: "test_container_2".to_string(),
            log: "This is a test log for container 2".to_string(),
            log_level: LogLevel::Error,
            created_at: Some(Utc::now().timestamp()),
            other_fields: Some(HashMap::new()),
        };

        // Insert the logs into the database
        ContainerLog::record(&pool, &log1).await.unwrap();
        ContainerLog::record(&pool, &log2).await.unwrap();

        let logs = ContainerLog::get_all_for_node(&pool, node_id).await.unwrap();

        assert_eq!(logs.len(), 2);
        assert_eq!(logs, vec![log1, log2]);

        Ok(())
    }

    #[sqlx::test(fixtures(
        "../../../backend/fixtures/organization.sql",
        "../../../backend/fixtures/node.sql"
    ))]
    async fn test_record_and_get_all_for_node(pool: PgPool) -> sqlx::Result<()> {
        let node_id = "0x00000000000000000000000000000000deadbeef".parse::<Address>().unwrap();
        let log = ContainerLog {
            node_id: Some(node_id),
            container_id: Some("container1".to_string()),
            container_name: "test_container".to_string(),
            log: "Test log".to_string(),
            log_level: LogLevel::Info,
            created_at: Some(Utc::now().timestamp()),
            other_fields: Some(HashMap::new()),
        };

        ContainerLog::record(&pool, &log).await.unwrap();
        let logs = ContainerLog::get_all_for_node(&pool, node_id).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0], log);

        Ok(())
    }

    #[sqlx::test(fixtures(
        "../../../backend/fixtures/organization.sql",
        "../../../backend/fixtures/node.sql"
    ))]
    async fn test_get_all_for_node_between_timestamps(pool: PgPool) -> sqlx::Result<()> {
        let node_id = "0x00000000000000000000000000000000deadbeef".parse::<Address>().unwrap();
        let now = Utc::now().timestamp();
        let log1 = ContainerLog {
            node_id: Some(node_id),
            container_id: Some("container1".to_string()),
            container_name: "test_container1".to_string(),
            log: "Test log 1".to_string(),
            log_level: LogLevel::Info,
            created_at: Some(now),
            other_fields: Some(HashMap::new()),
        };
        let log2 = ContainerLog {
            node_id: Some(node_id),
            container_id: Some("container2".to_string()),
            container_name: "test_container2".to_string(),
            log: "Test log 2".to_string(),
            log_level: LogLevel::Error,
            created_at: Some(now + 100),
            other_fields: Some(HashMap::new()),
        };

        ContainerLog::record(&pool, &log1).await.unwrap();
        ContainerLog::record(&pool, &log2).await.unwrap();

        let logs =
            ContainerLog::get_all_for_node_between_timestamps(&pool, node_id, now - 50, now + 50)
                .await
                .unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0], log1);

        Ok(())
    }

    #[sqlx::test(fixtures(
        "../../../backend/fixtures/organization.sql",
        "../../../backend/fixtures/node.sql"
    ))]
    async fn test_get_all_for_node_with_log_level(pool: PgPool) -> sqlx::Result<()> {
        let node_id = "0x00000000000000000000000000000000deadbeef".parse::<Address>().unwrap();
        let log1 = ContainerLog {
            node_id: Some(node_id),
            container_id: Some("container1".to_string()),
            container_name: "test_container1".to_string(),
            log: "Test log 1".to_string(),
            log_level: LogLevel::Info,
            created_at: Some(Utc::now().timestamp()),
            other_fields: Some(HashMap::new()),
        };
        let log2 = ContainerLog {
            node_id: Some(node_id),
            container_id: Some("container2".to_string()),
            container_name: "test_container2".to_string(),
            log: "Test log 2".to_string(),
            log_level: LogLevel::Error,
            created_at: Some(Utc::now().timestamp()),
            other_fields: Some(HashMap::new()),
        };

        ContainerLog::record(&pool, &log1).await.unwrap();
        ContainerLog::record(&pool, &log2).await.unwrap();

        let logs = ContainerLog::get_all_for_node_with_log_level(&pool, node_id, LogLevel::Info)
            .await
            .unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0], log1);

        Ok(())
    }

    #[sqlx::test(fixtures(
        "../../../backend/fixtures/organization.sql",
        "../../../backend/fixtures/node.sql"
    ))]
    async fn test_get_all_for_node_between_timestamps_with_log_level(
        pool: PgPool,
    ) -> sqlx::Result<()> {
        let node_id = "0x00000000000000000000000000000000deadbeef".parse::<Address>().unwrap();
        let now = Utc::now().timestamp();
        let log1 = ContainerLog {
            node_id: Some(node_id),
            container_id: Some("container1".to_string()),
            container_name: "test_container1".to_string(),
            log: "Test log 1".to_string(),
            log_level: LogLevel::Info,
            created_at: Some(now),
            other_fields: Some(HashMap::new()),
        };
        let log2 = ContainerLog {
            node_id: Some(node_id),
            container_id: Some("container2".to_string()),
            container_name: "test_container2".to_string(),
            log: "Test log 2".to_string(),
            log_level: LogLevel::Error,
            created_at: Some(now + 100),
            other_fields: Some(HashMap::new()),
        };

        ContainerLog::record(&pool, &log1).await.unwrap();
        ContainerLog::record(&pool, &log2).await.unwrap();

        let logs = ContainerLog::get_all_for_node_between_timestamps_with_log_level(
            &pool,
            node_id,
            now - 50,
            now + 150,
            LogLevel::Error,
        )
        .await
        .unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0], log2);

        Ok(())
    }
}
