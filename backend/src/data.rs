use crate::{db::metric::Metric, error::BackendError};

pub fn filter_metrics(metrics: &[Metric]) -> Result<Vec<Metric>, BackendError> {
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

fn filter_metrics_by_names(metrics: &[Metric], allowed_names: &[&str]) -> Vec<Metric> {
    metrics.iter().filter(|metric| allowed_names.contains(&metric.name.as_str())).cloned().collect()
}

fn find_running_avs(metrics: &[Metric]) -> Option<String> {
    metrics
        .iter()
        .find(|metric| metric.name.contains("running"))
        .and_then(|metric| metric.attributes.as_ref()?.get("avs").cloned())
}

const CONDENSED_EIGENDA_METRICS_NAMES: [&str; 6] = [
    "eigen_performance_score",
    "node_reachability_status",
    "cpu_usage",
    "disk_usage",
    "uptime",
    "ram_usage",
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

        let filtered_metrics = super::filter_metrics(&metrics)?;
        println!("{:#?}", filtered_metrics);
        assert!(filtered_metrics.len() == 7);
        Ok(())
    }
}
