use crate::error::DatabaseError;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use sqlx::{query, query_as, PgPool};
use std::{collections::HashMap, fmt::Display, str::FromStr};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, sqlx::Type, Deserialize, Serialize, ToSchema,
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
            "unknown" => Ok(LogLevel::Unknown),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow, PartialEq, Eq)]
pub struct ContainerLog {
    pub machine_id: Uuid,
    pub avs_name: String,
    pub log: String,
    pub log_level: LogLevel,
    pub created_at: Option<i64>,
    #[serde(flatten, deserialize_with = "deserialize_other_fields")]
    pub other_fields: Option<HashMap<String, String>>,
}

impl Display for ContainerLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ContainerLog {{ machine_id: {}, avs_name: {}, log: {}, log_level: {:?}, created_at: {:?}, other_fields: {:?} }}",
            self.machine_id, self.avs_name, self.log, self.log_level, self.created_at, self.other_fields
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, sqlx::FromRow, PartialEq, Eq)]
pub struct ContainerDbLog {
    pub machine_id: Uuid,
    pub avs_name: String,
    pub log: String,
    pub log_level: LogLevel,
    pub created_at: NaiveDateTime,
    pub other_fields: Option<sqlx::types::Json<HashMap<String, String>>>,
}

impl From<ContainerDbLog> for ContainerLog {
    fn from(value: ContainerDbLog) -> Self {
        Self {
            machine_id: value.machine_id,
            avs_name: value.avs_name.clone(),
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
            machine_id: value.machine_id,
            avs_name: value.avs_name.clone(),
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
    ) -> Result<(), DatabaseError> {
        // cast timestamp to NaiveDateTime or get current time
        let created_at =
            DateTime::from_timestamp(log.created_at.unwrap_or_else(|| Utc::now().timestamp()), 0)
                .expect("Could not construct datetime")
                .naive_utc();
        query!(
            "INSERT INTO log (machine_id, avs_name, log, log_level, created_at, other_fields) VALUES ($1, $2, $3, $4, $5, $6)",
            log.machine_id,
            log.avs_name,
            log.log,
            log.log_level as LogLevel,
            created_at,
            log.other_fields.as_ref().map(|v| json!(v)),
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_all_for_machine(
        pool: &PgPool,
        machine_id: Uuid,
    ) -> Result<Vec<ContainerLog>, DatabaseError> {
        let db_logs: Vec<ContainerDbLog> = query_as!(
            ContainerDbLog,
            r#"SELECT machine_id, avs_name, log, log_level AS "log_level!: LogLevel", created_at, other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>" FROM log WHERE machine_id = $1"#,
            machine_id
        )
        .fetch_all(pool)
        .await?;

        let logs = db_logs.into_iter().map(ContainerLog::from).collect::<Vec<_>>();
        Ok(logs)
    }

    pub async fn get_all_for_avs(
        pool: &PgPool,
        machine_id: Uuid,
        avs_name: &str,
        from: Option<i64>,
        to: Option<i64>,
        log_level: Option<LogLevel>,
    ) -> Result<Vec<ContainerLog>, DatabaseError> {
        let db_logs: Vec<ContainerDbLog> =
            match (from, to, log_level) {
                (Some(from), Some(to), Some(log_level)) => query_as!(
                        ContainerDbLog,
                        r#"SELECT machine_id, avs_name, log,
                             log_level AS "log_level!: LogLevel", created_at,
                             other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>"
                           FROM
                             log
                           WHERE
                             machine_id = $1
                             AND avs_name = $2
                             AND created_at >= $3
                             AND created_at <= $4
                             AND log_level = $5
                           ORDER BY created_at"#,
                        machine_id,
                        avs_name,
                        DateTime::from_timestamp(from, 0).expect("Invalid timestamp").naive_utc(),
                        DateTime::from_timestamp(to, 0).expect("Invalid timestamp").naive_utc(),
                        log_level as LogLevel,
                        ).fetch_all(pool).await,
                (Some(from), Some(to), None) => query_as!(
                        ContainerDbLog,
                        r#"SELECT machine_id, avs_name, log,
                             log_level AS "log_level!: LogLevel", created_at,
                             other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>"
                           FROM
                             log
                           WHERE
                             machine_id = $1
                             AND avs_name = $2
                             AND created_at >= $3
                             AND created_at <= $4
                           ORDER BY created_at"#,
                        machine_id,
                        avs_name,
                        DateTime::from_timestamp(from, 0).expect("Invalid timestamp").naive_utc(),
                        DateTime::from_timestamp(to, 0).expect("Invalid timestamp").naive_utc(),
                        ).fetch_all(pool).await,
                (Some(from), None, Some(log_level)) => query_as!(
                        ContainerDbLog,
                         r#"SELECT machine_id, avs_name, log,
                             log_level AS "log_level!: LogLevel", created_at,
                             other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>"
                           FROM
                             log
                           WHERE
                             machine_id = $1
                             AND avs_name = $2
                             AND created_at >= $3
                             AND log_level = $4
                           ORDER BY created_at"#,
                        machine_id,
                        avs_name,
                        DateTime::from_timestamp(from, 0).expect("Invalid timestamp").naive_utc(),
                        log_level as LogLevel,
                        ).fetch_all(pool).await,
                (Some(from), None, None) => query_as!(
                        ContainerDbLog,
                         r#"SELECT machine_id, avs_name, log,
                             log_level AS "log_level!: LogLevel", created_at,
                             other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>"
                           FROM
                             log
                           WHERE
                             machine_id = $1
                             AND avs_name = $2
                             AND created_at >= $3
                           ORDER BY created_at"#,
                        machine_id,
                        avs_name,
                        DateTime::from_timestamp(from, 0).expect("Invalid timestamp").naive_utc(),
                        ).fetch_all(pool).await,
                (None, Some(to), None) => query_as!(
                        ContainerDbLog,
                        r#"SELECT machine_id, avs_name, log,
                             log_level AS "log_level!: LogLevel", created_at,
                             other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>"
                           FROM
                             log
                           WHERE
                             machine_id = $1
                             AND avs_name = $2
                             AND created_at <= $3
                           ORDER BY created_at"#,
                        machine_id,
                        avs_name,
                        DateTime::from_timestamp(to, 0).expect("Invalid timestamp").naive_utc(),
                        ).fetch_all(pool).await,
                (None, Some(to), Some(log_level)) => query_as!(
                        ContainerDbLog,
                        r#"SELECT machine_id, avs_name, log,
                             log_level AS "log_level!: LogLevel", created_at,
                             other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>"
                           FROM
                             log
                           WHERE
                             machine_id = $1
                             AND avs_name = $2
                             AND created_at <= $3
                             AND log_level = $4"#,
                        machine_id,
                        avs_name,
                        DateTime::from_timestamp(to, 0).expect("Invalid timestamp").naive_utc(),
                        log_level as LogLevel,
                        ).fetch_all(pool).await,
                (None, None, Some(log_level)) => query_as!(
                        ContainerDbLog,
                         r#"SELECT machine_id, avs_name, log,
                             log_level AS "log_level!: LogLevel", created_at,
                             other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>"
                           FROM
                             log
                           WHERE
                             machine_id = $1
                             AND avs_name = $2
                             AND log_level = $3"#,
                        machine_id,
                        avs_name,
                        log_level as LogLevel,
                        ).fetch_all(pool).await,
                (None, None, None) => query_as!(
                        ContainerDbLog,
                         r#"SELECT machine_id, avs_name, log,
                             log_level AS "log_level!: LogLevel", created_at,
                             other_fields as "other_fields: sqlx::types::Json<HashMap<String,String>>"
                           FROM
                             log
                           WHERE
                             machine_id = $1
                             AND avs_name = $2"#,
                        machine_id,
                        avs_name,
                        ).fetch_all(pool).await,
            }?;

        let logs = db_logs.into_iter().map(ContainerLog::from).collect::<Vec<_>>();
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

#[cfg(feature = "db_tests")]
#[cfg(test)]
mod logs_db_tests {
    use ivynet_core::{
        docker::logs::{find_log_level, find_or_create_log_timestamp, sanitize_log},
        ethers::types::Address,
        node_type::NodeType,
    };

    use crate::db::{Account, Avs, Client, Machine, Organization, Role};

    use super::*;

    const INVALID_UTF8_LOG: &str = "2024-11-28 14:29:20 eigenda-native-node  | Nov 28 20:29:20.271 ERR node/node.go:775 Reachability check - dispersal socket is UNREACHABLE component=Node socket=216.254.247.80:32005 error from daemon in stream: Error grabbing logs: invalid character '\x00' looking for beginning of value";

    #[sqlx::test()]
    async fn test_post_log(pool: PgPool) -> sqlx::Result<()> {
        // Setup organization
        Organization::new(&pool, "Test Org", true).await.unwrap();
        let _ = Organization::get(&pool, 1).await.expect("Failed to fetch org 1");

        // Setup client
        let client_id = Address::from_slice(&[8; 20]);
        let account = Account {
            user_id: 1234567890,
            organization_id: 1,
            email: "db_test_user@ivynet.dev".to_string(),
            password: "db_test_1234".to_string(),
            role: Role::User,
            created_at: None,
            updated_at: None,
        };

        Client::create(&pool, &account, &client_id).await.unwrap();

        // Setup Machine
        let name = "test_machine";
        let machine_id = Uuid::new_v4();

        Machine::create(&pool, &client_id, name, machine_id).await.unwrap();

        // Setup Avs
        let avs_name = "test_avs";
        let avs_type = NodeType::Unknown;
        let version_hash = "test_hash";

        Avs::record_avs_data_from_client(&pool, machine_id, avs_name, &avs_type, version_hash)
            .await
            .unwrap();

        // log data
        let sanitized = sanitize_log(INVALID_UTF8_LOG);
        let log_level = LogLevel::from_str(&find_log_level(&sanitized)).unwrap();
        let created_at = Some(find_or_create_log_timestamp(&sanitized));

        let unsanitized_log = ContainerLog {
            machine_id,
            avs_name: avs_name.to_string(),
            log: INVALID_UTF8_LOG.to_string(),
            log_level,
            created_at,
            other_fields: None,
        };

        let try_record_unsanitized = ContainerLog::record(&pool, &unsanitized_log).await;
        assert!(try_record_unsanitized.is_err());

        let sanitized_log = ContainerLog {
            machine_id,
            avs_name: avs_name.to_string(),
            log: sanitized,
            log_level,
            created_at,
            other_fields: None,
        };

        let try_record_sanitized = ContainerLog::record(&pool, &sanitized_log).await;
        assert!(try_record_sanitized.is_ok());

        let logs =
            ContainerLog::get_all_for_machine(&pool, sanitized_log.machine_id).await.unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0], sanitized_log);

        Ok(())
    }
}

// #[cfg(feature = "db_tests")]
// #[cfg(test)]
// mod logs_db_tests {
//     use super::*;
//     use crate::db::node::DbNode;
//     use chrono::Utc;
//     use ivynet_core::ethers::types::Address;
//
//     #[sqlx::test(fixtures(
//         "../../../backend/fixtures/organization.sql",
//         "../../../backend/fixtures/node.sql"
//     ))]
//     async fn test_insert_and_retrieve_logs(pool: PgPool) -> sqlx::Result<()> {
//         let node_id = "0x00000000000000000000000000000000deadbeef".parse::<Address>().unwrap();
//
//         let node = DbNode::get(&pool, &node_id).await;
//
//         let log1 = ContainerLog {
//             node_id: Some(node_id),
//             container_id: Some("container1".to_string()),
//             container_name: "test_container_1".to_string(),
//             log: "This is a test log for container 1".to_string(),
//             log_level: LogLevel::Info, // Assuming you have a LogLevel enum
//             created_at: Some(Utc::now().timestamp()),
//             other_fields: Some(HashMap::new()),
//         };
//
//         // wait for two seconds
//         std::thread::sleep(std::time::Duration::from_secs(2));
//
//         let log2 = ContainerLog {
//             node_id: Some(node_id),
//             container_id: Some("container2".to_string()),
//             container_name: "test_container_2".to_string(),
//             log: "This is a test log for container 2".to_string(),
//             log_level: LogLevel::Error,
//             created_at: Some(Utc::now().timestamp()),
//             other_fields: Some(HashMap::new()),
//         };
//
//         // Insert the logs into the database
//         ContainerLog::record(&pool, &log1).await.unwrap();
//         ContainerLog::record(&pool, &log2).await.unwrap();
//
//         let logs = ContainerLog::get_all_for_node(&pool, node_id).await.unwrap();
//
//         assert_eq!(logs.len(), 2);
//         assert_eq!(logs, vec![log1, log2]);
//
//         Ok(())
//     }
//
//     #[sqlx::test(fixtures(
//         "../../../backend/fixtures/organization.sql",
//         "../../../backend/fixtures/node.sql"
//     ))]
//     async fn test_record_and_get_all_for_node(pool: PgPool) -> sqlx::Result<()> {
//         let node_id = "0x00000000000000000000000000000000deadbeef".parse::<Address>().unwrap();
//         let log = ContainerLog {
//             node_id: Some(node_id),
//             container_id: Some("container1".to_string()),
//             container_name: "test_container".to_string(),
//             log: "Test log".to_string(),
//             log_level: LogLevel::Info,
//             created_at: Some(Utc::now().timestamp()),
//             other_fields: Some(HashMap::new()),
//         };
//
//         ContainerLog::record(&pool, &log).await.unwrap();
//         let logs = ContainerLog::get_all_for_node(&pool, node_id).await.unwrap();
//         assert_eq!(logs.len(), 1);
//         assert_eq!(logs[0], log);
//
//         Ok(())
//     }
//
//     #[sqlx::test(fixtures(
//         "../../../backend/fixtures/organization.sql",
//         "../../../backend/fixtures/node.sql"
//     ))]
//     async fn test_get_all_for_node_between_timestamps(pool: PgPool) -> sqlx::Result<()> {
//         let node_id = "0x00000000000000000000000000000000deadbeef".parse::<Address>().unwrap();
//         let now = Utc::now().timestamp();
//         let log1 = ContainerLog {
//             node_id: Some(node_id),
//             container_id: Some("container1".to_string()),
//             container_name: "test_container1".to_string(),
//             log: "Test log 1".to_string(),
//             log_level: LogLevel::Info,
//             created_at: Some(now),
//             other_fields: Some(HashMap::new()),
//         };
//         let log2 = ContainerLog {
//             node_id: Some(node_id),
//             container_id: Some("container2".to_string()),
//             container_name: "test_container2".to_string(),
//             log: "Test log 2".to_string(),
//             log_level: LogLevel::Error,
//             created_at: Some(now + 100),
//             other_fields: Some(HashMap::new()),
//         };
//
//         ContainerLog::record(&pool, &log1).await.unwrap();
//         ContainerLog::record(&pool, &log2).await.unwrap();
//
//         let logs =
//             ContainerLog::get_all_for_node_between_timestamps(&pool, node_id, now - 50, now + 50)
//                 .await
//                 .unwrap();
//         assert_eq!(logs.len(), 1);
//         assert_eq!(logs[0], log1);
//
//         Ok(())
//     }
//
//     #[sqlx::test(fixtures(
//         "../../../backend/fixtures/organization.sql",
//         "../../../backend/fixtures/node.sql"
//     ))]
//     async fn test_get_all_for_node_with_log_level(pool: PgPool) -> sqlx::Result<()> {
//         let node_id = "0x00000000000000000000000000000000deadbeef".parse::<Address>().unwrap();
//         let log1 = ContainerLog {
//             node_id: Some(node_id),
//             container_id: Some("container1".to_string()),
//             container_name: "test_container1".to_string(),
//             log: "Test log 1".to_string(),
//             log_level: LogLevel::Info,
//             created_at: Some(Utc::now().timestamp()),
//             other_fields: Some(HashMap::new()),
//         };
//         let log2 = ContainerLog {
//             node_id: Some(node_id),
//             container_id: Some("container2".to_string()),
//             container_name: "test_container2".to_string(),
//             log: "Test log 2".to_string(),
//             log_level: LogLevel::Error,
//             created_at: Some(Utc::now().timestamp()),
//             other_fields: Some(HashMap::new()),
//         };
//
//         ContainerLog::record(&pool, &log1).await.unwrap();
//         ContainerLog::record(&pool, &log2).await.unwrap();
//
//         let logs = ContainerLog::get_all_for_node_with_log_level(&pool, node_id, LogLevel::Info)
//             .await
//             .unwrap();
//         assert_eq!(logs.len(), 1);
//         assert_eq!(logs[0], log1);
//
//         Ok(())
//     }
//
//     #[sqlx::test(fixtures(
//         "../../../backend/fixtures/organization.sql",
//         "../../../backend/fixtures/node.sql"
//     ))]
//     async fn test_get_all_for_node_between_timestamps_with_log_level(
//         pool: PgPool,
//     ) -> sqlx::Result<()> {
//         let node_id = "0x00000000000000000000000000000000deadbeef".parse::<Address>().unwrap();
//         let now = Utc::now().timestamp();
//         let log1 = ContainerLog {
//             node_id: Some(node_id),
//             container_id: Some("container1".to_string()),
//             container_name: "test_container1".to_string(),
//             log: "Test log 1".to_string(),
//             log_level: LogLevel::Info,
//             created_at: Some(now),
//             other_fields: Some(HashMap::new()),
//         };
//         let log2 = ContainerLog {
//             node_id: Some(node_id),
//             container_id: Some("container2".to_string()),
//             container_name: "test_container2".to_string(),
//             log: "Test log 2".to_string(),
//             log_level: LogLevel::Error,
//             created_at: Some(now + 100),
//             other_fields: Some(HashMap::new()),
//         };
//
//         ContainerLog::record(&pool, &log1).await.unwrap();
//         ContainerLog::record(&pool, &log2).await.unwrap();
//
//         let logs = ContainerLog::get_all_for_node_between_timestamps_with_log_level(
//             &pool,
//             node_id,
//             now - 50,
//             now + 150,
//             LogLevel::Error,
//         )
//         .await
//         .unwrap();
//         assert_eq!(logs.len(), 1);
//         assert_eq!(logs[0], log2);
//
//         Ok(())
//     }
// }
