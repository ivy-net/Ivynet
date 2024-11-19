use std::collections::HashMap;

use ivynet_core::{ethers::types::Chain, node_type::NodeType};

use semver::Version;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    db::{
        avs_version::{AvsID, VersionData},
        metric::Metric,
    },
    error::BackendError,
};

const RUNNING_METRIC: &str = "running";
const EIGEN_PERFORMANCE_METRIC: &str = "eigen_performance_score";
const IDLE_MINUTES_THRESHOLD: i64 = 15;

const EIGEN_PERFORMANCE_HEALTHY_THRESHOLD: f64 = 80.0;

#[derive(Serialize, ToSchema, Clone, Debug)]
pub enum NodeStatus {
    Healthy,
    Unhealthy,
    Idle,
    Error,
    UpdateNeeded,
}

/// Condense list of metrics into a smaller list of metrics for the frontend
pub fn condense_metrics(metrics: &[Metric]) -> Result<Vec<Metric>, BackendError> {
    let avs = find_running_avs(metrics).ok_or(BackendError::NoRunningAvsFound(
        "No running AVS found when searching for condensed metrics".to_owned(),
    ))?;

    match avs.as_str() {
        "eigenda" => Ok(filter_metrics_by_names(metrics, &CONDENSED_EIGENDA_METRICS_NAMES)),
        _ => Err(BackendError::CondensedMetricsNotFound(format!(
            "No condensed metrics found for AVS: {}, use the /metrics/all endpoint instead",
            avs
        ))),
    }
}

/// Filter the metrics by the given names.
fn filter_metrics_by_names(metrics: &[Metric], allowed_names: &[&str]) -> Vec<Metric> {
    metrics.iter().filter(|metric| allowed_names.contains(&metric.name.as_str())).cloned().collect()
}

/// Find the name of the running AVS.
fn find_running_avs(metrics: &[Metric]) -> Option<String> {
    metrics
        .iter()
        .find(|metric| metric.name.contains("running"))
        .and_then(|metric| metric.attributes.as_ref()?.get("avs").cloned())
}

/// Categorize the running nodes into two groups: avs running and idle.
/// FIXME: This function is dependent on running metric impl, is there ever
/// a case where running metric would be 0 now? I think yes, monitoring but
/// no actual node running - could signal a broken node
pub fn categorize_running_nodes(
    node_metrics_map: HashMap<Uuid, HashMap<String, Metric>>,
) -> (Vec<Uuid>, Vec<Uuid>) {
    let mut running_nodes = Vec::new();
    let mut idle_nodes = Vec::new();

    node_metrics_map.iter().for_each(|(node_id, metrics_map)| {
        let is_running = metrics_map
            .get(RUNNING_METRIC)
            .and_then(|metric| {
                (metric.value > 0.0).then(|| {
                    metric.created_at.map(|datetime| {
                        let now = chrono::Utc::now().naive_utc();
                        now.signed_duration_since(datetime).num_minutes() < IDLE_MINUTES_THRESHOLD
                    })
                })
            })
            .flatten()
            .unwrap_or(false);

        if is_running {
            running_nodes.push(*node_id);
        } else {
            idle_nodes.push(*node_id);
        }
    });

    (running_nodes, idle_nodes)
}

/// Categorize the running nodes into two groups: healthy and unhealthy.
pub fn categorize_node_health(
    running_nodes: Vec<Uuid>,
    node_metrics_map: HashMap<Uuid, HashMap<String, Metric>>,
) -> (Vec<Uuid>, Vec<Uuid>) {
    let mut healthy_nodes = Vec::new();
    let mut low_performance_nodes = Vec::new();
    for node in running_nodes {
        if let Some(metrics_map) = node_metrics_map.get(&node) {
            if let Some(performance_metric) = metrics_map.get(EIGEN_PERFORMANCE_METRIC) {
                if performance_metric.value >= EIGEN_PERFORMANCE_HEALTHY_THRESHOLD {
                    healthy_nodes.push(node);
                } else {
                    low_performance_nodes.push(node);
                }
            }
        }
    }

    (healthy_nodes, low_performance_nodes)
}

/// Get nodes that need to be updated.
pub fn categorize_updateable_nodes(
    running_nodes: Vec<Uuid>,
    node_metrics_map: HashMap<Uuid, HashMap<String, Metric>>,
    avs_version_map: HashMap<AvsID, VersionData>,
) -> (Vec<Uuid>, Vec<Uuid>) {
    let mut updateable = Vec::new();
    let mut outdated = Vec::new();
    running_nodes
        .iter()
        .filter_map(|&node| {
            let metrics = node_metrics_map.get(&node)?;
            let running_metric = metrics.get(RUNNING_METRIC)?;
            let metric_attributes = running_metric.attributes.as_ref()?;

            let avs = metric_attributes.get("avs")?;
            let chain = metric_attributes.get("chain")?;
            let version = metric_attributes.get("version")?;

            let avs_id = AvsID {
                avs_name: NodeType::try_from(avs.as_str()),
                chain: chain.parse::<Chain>().ok()?,
            };
            let current_version = Version::parse(version).ok()?;

            let version_data = avs_version_map.get(&avs_id)?.clone();
            if version_data.latest_version <= current_version {
                return None;
            }

            updateable.push(node);

            if let Some(breaking) = version_data.breaking_change_version {
                if current_version < breaking {
                    outdated.push(node);
                }
            }

            Some(())
        })
        .count(); // consume iterator

    (updateable, outdated)
}

/// Look up NodeStatus of a specific node
pub fn get_node_status_from_id(
    node_id: Uuid,
    node_metrics_map: &HashMap<Uuid, HashMap<String, Metric>>,
) -> NodeStatus {
    if let Some(metrics_map) = node_metrics_map.get(&node_id) {
        return get_node_status(metrics_map.clone());
    }

    NodeStatus::Error
}

pub fn get_node_status(metrics: HashMap<String, Metric>) -> NodeStatus {
    match (
        metrics.get(RUNNING_METRIC).as_ref().map(|s| s.value > 0.0),
        metrics.get(EIGEN_PERFORMANCE_METRIC),
    ) {
        (Some(true), Some(performance)) => {
            if performance.value > EIGEN_PERFORMANCE_HEALTHY_THRESHOLD {
                NodeStatus::Healthy
            } else {
                NodeStatus::Unhealthy
            }
        }
        (Some(true), None) => NodeStatus::Unhealthy,
        (Some(false), _) => NodeStatus::Idle,
        _ => NodeStatus::Error,
    }
}

const CONDENSED_EIGENDA_METRICS_NAMES: [&str; 7] = [
    "eigen_performance_score",
    "node_reachability_status",
    "cpu_usage",
    "disk_usage",
    "uptime",
    "ram_usage",
    "running",
];

#[cfg(test)]
mod data_filtering_tests {
    use crate::db::avs_version::VersionData;

    use super::*;
    use std::{fs::File, io::BufReader};

    use chrono::NaiveDateTime;
    use ivynet_core::ethers::types::Chain;

    fn create_metric(
        value: f64,
        created_at: Option<NaiveDateTime>,
        attributes: Option<HashMap<String, String>>,
    ) -> Metric {
        Metric {
            value,
            created_at,
            machine_id: Uuid::new_v4(),
            avs_name: Some(NodeType::EigenDA.to_string()),
            name: "JimTheComputer".to_owned(),
            attributes,
        }
    }

    fn create_metric_with_version_attributes(avs: &str, version: &str) -> Metric {
        create_metric(
            1.0,
            None,
            Some(HashMap::from([
                ("avs".to_string(), avs.to_string()),
                ("chain".to_string(), Chain::Holesky.to_string()),
                ("version".to_string(), version.to_string()),
            ])),
        )
    }

    fn create_version_data(version: &str, breaking_version: Option<&str>) -> VersionData {
        VersionData {
            latest_version: Version::parse(version).unwrap(),
            breaking_change_version: breaking_version.map(|v| Version::parse(v).unwrap()),
            breaking_change_datetime: breaking_version.map(|_| chrono::Utc::now().naive_utc()),
        }
    }

    fn create_id(avs_name: &str) -> AvsID {
        AvsID { avs_name: NodeType::from(avs_name), chain: Chain::Holesky }
    }

    fn load_metrics_json(file_path: &str) -> Result<Vec<Metric>, Box<dyn std::error::Error>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let metrics: Vec<Metric> = serde_json::from_reader(reader)?;
        Ok(metrics)
    }

    #[test]
    fn test_find_avs_name() -> Result<(), Box<dyn std::error::Error>> {
        let metrics: Vec<Metric> = load_metrics_json("test/json/eigenda_metrics.json")?;

        let name = super::find_running_avs(&metrics).unwrap();
        assert_eq!(name, "eigenda");
        Ok(())
    }

    #[test]
    fn test_filter_metrics() -> Result<(), Box<dyn std::error::Error>> {
        let metrics: Vec<Metric> = load_metrics_json("test/json/eigenda_metrics.json")?;

        let filtered_metrics = super::condense_metrics(&metrics)?;
        println!("{:#?}", filtered_metrics);
        assert!(filtered_metrics.len() == 8);
        Ok(())
    }

    #[test]
    fn test_categorize_running_nodes() {
        let now = chrono::Utc::now().naive_utc();
        let recent = now - chrono::Duration::minutes(IDLE_MINUTES_THRESHOLD - 1);
        let old = now - chrono::Duration::minutes(IDLE_MINUTES_THRESHOLD + 1);

        let mut node_metrics_map = HashMap::new();

        // Running node
        let mut metrics1 = HashMap::new();
        metrics1.insert(RUNNING_METRIC.to_string(), create_metric(1.0, Some(recent), None));
        node_metrics_map.insert(Uuid::from_u128(1), metrics1);

        // Idle node (value = 0)
        let mut metrics2 = HashMap::new();
        metrics2.insert(RUNNING_METRIC.to_string(), create_metric(0.0, Some(recent), None));
        node_metrics_map.insert(Uuid::from_u128(2), metrics2);

        // Idle node (old timestamp)
        let mut metrics3 = HashMap::new();
        metrics3.insert(RUNNING_METRIC.to_string(), create_metric(1.0, Some(old), None));
        node_metrics_map.insert(Uuid::from_u128(3), metrics3);

        // Idle node (no timestamp)
        let mut metrics4 = HashMap::new();
        metrics4.insert(RUNNING_METRIC.to_string(), create_metric(1.0, None, None));
        node_metrics_map.insert(Uuid::from_u128(4), metrics4);

        // Node without RUNNING_METRIC
        let metrics5 = HashMap::new();
        node_metrics_map.insert(Uuid::from_u128(5), metrics5);

        let (running_nodes, idle_nodes) = categorize_running_nodes(node_metrics_map);

        assert_eq!(running_nodes, vec![Uuid::from_u128(1)]);
        assert!(idle_nodes.len() == 4);
        assert!(idle_nodes.contains(&Uuid::from_u128(2)));
        assert!(idle_nodes.contains(&Uuid::from_u128(3)));
        assert!(idle_nodes.contains(&Uuid::from_u128(4)));
        assert!(idle_nodes.contains(&Uuid::from_u128(5)));
    }

    #[test]
    fn test_categorize_updateable_nodes() {
        let node1 = Uuid::from_u128(1);
        let node2 = Uuid::from_u128(2);
        let node3 = Uuid::from_u128(3);

        let running_nodes = vec![node1, node2, node3];

        let mut node_metrics_map = HashMap::new();
        node_metrics_map.insert(
            node1,
            HashMap::from([(
                RUNNING_METRIC.to_string(),
                create_metric_with_version_attributes("eigenda", "2.0.0"),
            )]),
        );
        node_metrics_map.insert(
            node2,
            HashMap::from([(
                RUNNING_METRIC.to_string(),
                create_metric_with_version_attributes("lagrange", "2.0.0"),
            )]),
        );
        node_metrics_map.insert(
            node3,
            HashMap::from([(
                RUNNING_METRIC.to_string(),
                create_metric_with_version_attributes("eigenda", "1.5.0"),
            )]),
        );

        let avs_version_map = HashMap::from([
            (create_id("eigenda"), create_version_data("2.0.0", Some("1.8.0"))),
            (create_id("lagrange"), create_version_data("2.1.0", None)),
        ]);

        let (updateable_nodes, outdated_nodes) =
            categorize_updateable_nodes(running_nodes, node_metrics_map, avs_version_map);

        assert_eq!(updateable_nodes.len(), 2);
        assert!(updateable_nodes.contains(&node2));
        assert!(updateable_nodes.contains(&node3));
        assert!(!updateable_nodes.contains(&node1));

        assert_eq!(outdated_nodes.len(), 1);
        assert!(outdated_nodes.contains(&node3));
    }

    #[test]
    fn test_no_updateable_nodes() {
        let node1 = Uuid::from_u128(1);

        let running_nodes = vec![node1];

        let mut node_metrics_map = HashMap::new();
        node_metrics_map.insert(
            node1,
            HashMap::from([(
                RUNNING_METRIC.to_string(),
                create_metric_with_version_attributes("eigenda", "2.0.0"),
            )]),
        );

        let avs_version_map =
            HashMap::from([(create_id("eigenda"), create_version_data("2.0.0", None))]);

        let (updateable_nodes, outdated_nodes) =
            categorize_updateable_nodes(running_nodes, node_metrics_map, avs_version_map);

        assert_eq!(updateable_nodes.len(), 0);
        assert_eq!(outdated_nodes.len(), 0);
    }

    #[test]
    fn test_missing_avs_or_version() {
        let node1 = Uuid::from_u128(1);
        let node2 = Uuid::from_u128(2);

        let running_nodes = vec![node1, node2];

        let mut node_metrics_map = HashMap::new();
        node_metrics_map.insert(
            node1,
            HashMap::from([(
                RUNNING_METRIC.to_string(),
                create_metric_with_version_attributes("eigenda", "1.0.0"),
            )]),
        );
        node_metrics_map.insert(
            node2,
            HashMap::from([(
                RUNNING_METRIC.to_string(),
                create_metric_with_version_attributes("lagrange", "0.0.0"),
            )]),
        );

        let avs_version_map =
            HashMap::from([(create_id("eigenda"), create_version_data("2.0.0", None))]);

        let (updateable_nodes, outdated_nodes) =
            categorize_updateable_nodes(running_nodes, node_metrics_map, avs_version_map);

        println!("{:?}", updateable_nodes);

        assert_eq!(updateable_nodes.len(), 1);
        assert!(updateable_nodes.contains(&node1));
        assert_eq!(outdated_nodes.len(), 0);
    }

    #[test]
    fn test_invalid_version_string() {
        let node1 = Uuid::from_u128(1);

        let running_nodes = vec![node1];

        let mut node_metrics_map = HashMap::new();
        node_metrics_map.insert(
            node1,
            HashMap::from([(
                RUNNING_METRIC.to_string(),
                create_metric_with_version_attributes("eigenda", "invalid"),
            )]),
        );

        let avs_version_map =
            HashMap::from([(create_id("eigenda"), create_version_data("2.0.0", None))]);

        let (updateable_nodes, outdated_nodes) =
            categorize_updateable_nodes(running_nodes, node_metrics_map, avs_version_map);

        assert_eq!(updateable_nodes.len(), 0);
        assert_eq!(outdated_nodes.len(), 0);
    }
}
