use std::collections::HashMap;

use ivynet_core::ethers::types::H160;
use serde::Serialize;
use utoipa::ToSchema;

use crate::{db::metric::Metric, error::BackendError};

const RUNNING_METRIC: &str = "running";
const EIGEN_PERFORMANCE_METRIC: &str = "eigen_performance_score";

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
pub fn categorize_running_nodes(
    node_metrics_map: HashMap<H160, HashMap<String, Metric>>,
) -> (Vec<H160>, Vec<H160>) {
    let mut running_nodes: Vec<H160> = Vec::new();
    let mut idle_nodes: Vec<H160> = Vec::new();

    node_metrics_map.iter().for_each(|(node_id, metrics_map)| {
        if let Some(metric) = metrics_map.get(RUNNING_METRIC) {
            if metric.value > 0.0 {
                running_nodes.push(*node_id);
            } else {
                idle_nodes.push(*node_id);
            }
        } else {
            idle_nodes.push(*node_id);
        }
    });

    (running_nodes, idle_nodes)
}

/// Categorize the running nodes into two groups: healthy and unhealthy.
pub fn categorize_node_health(
    running_nodes: Vec<H160>,
    node_metrics_map: HashMap<H160, HashMap<String, Metric>>,
) -> (Vec<H160>, Vec<H160>) {
    let mut healthy_machines = Vec::new();
    let mut unhealthy_machines = Vec::new();
    for node in running_nodes {
        if let Some(metrics_map) = node_metrics_map.get(&node) {
            if let Some(performance_metric) = metrics_map.get(EIGEN_PERFORMANCE_METRIC) {
                if performance_metric.value >= EIGEN_PERFORMANCE_HEALTHY_THRESHOLD {
                    healthy_machines.push(node);
                } else {
                    unhealthy_machines.push(node);
                }
            }
        }
    }

    (healthy_machines, unhealthy_machines)
}

/// Look up NodeStatus of a specific node
pub fn get_node_status_from_id(
    node_id: H160,
    node_metrics_map: &HashMap<H160, HashMap<String, Metric>>,
) -> NodeStatus {
    if let Some(metrics_map) = node_metrics_map.get(&node_id) {
        return get_node_status(metrics_map.clone())
    }

    NodeStatus::Error
}

pub fn get_node_status(metrics: HashMap<String, Metric>) -> NodeStatus {
    if let Some(running_metric) = metrics.get(RUNNING_METRIC) {
        if running_metric.value > 0.0 {
            if let Some(performance_metric) = metrics.get(EIGEN_PERFORMANCE_METRIC) {
                if performance_metric.value >= EIGEN_PERFORMANCE_HEALTHY_THRESHOLD {
                    return NodeStatus::Healthy;
                } else {
                    return NodeStatus::Unhealthy;
                }
            }
        } else {
            return NodeStatus::Idle;
        }
    }
    NodeStatus::Error
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
mod metrics_filtering_tests {
    use super::*;
    use std::{fs::File, io::BufReader};

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
}
