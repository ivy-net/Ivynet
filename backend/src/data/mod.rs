use crate::{db::metric::Metric, error::BackendError};

pub fn filter_metrics(metrics: &[Metric]) -> Result<Vec<Metric>, BackendError> {
    let avs = find_running_avs(metrics).ok_or_else(|| {
        BackendError::NoRunningAvsFound(
            "No running AVS found when searching for condensed metrics".to_string(),
        )
    })?;

    match avs.as_str() {
        "eigenda" => Ok(filter_metrics_by_names(metrics, &condensed_eigenda_metrics_names())),
        _ => Err(BackendError::CondensedMetricsNotFound(format!(
            "No condensed metrics found for AVS: {}, use the /metrics/all endpoint instead",
            avs
        ))),
    }
}

fn filter_metrics_by_names(metrics: &[Metric], allowed_names: &[&str]) -> Vec<Metric> {
    metrics
        .iter()
        .filter(|metric| allowed_names.iter().any(|&name| metric.name.contains(name)))
        .cloned()
        .collect()
}

fn find_running_avs(metrics: &[Metric]) -> Option<String> {
    metrics
        .iter()
        .find(|metric| metric.name.contains("running"))
        .and_then(|metric| metric.attributes.as_ref()?.get("avs").cloned())
}

fn condensed_eigenda_metrics_names() -> Vec<&'static str> {
    vec![
        "eigen_performance_score",
        "node_reachability_status",
        "cpu_usage",
        "disk_usage",
        "uptime",
        "ram_usage",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::BufReader};

    fn load_metrics_json(file_path: &str) -> Result<Vec<Metric>, Box<dyn std::error::Error>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let metrics: Vec<Metric> = serde_json::from_reader(reader)?;
        Ok(metrics)
    }

    #[test]
    fn test_find_avs_name() {
        let metrics: Vec<Metric> = load_metrics_json("test/json/eigenda_metrics.json").unwrap();

        let name = super::find_running_avs(&metrics).unwrap();
        assert_eq!(name, "eigenda");
    }

    #[test]
    fn test_filter_metrics() {
        let metrics: Vec<Metric> = load_metrics_json("test/json/eigenda_metrics.json").unwrap();

        let filtered_metrics = super::filter_metrics(&metrics).unwrap();
        println!("{:#?}", filtered_metrics);
        assert_eq!(filtered_metrics.len(), 7);
    }
}
