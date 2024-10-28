use std::path::PathBuf;

use docker_compose_types::{Compose, Labels, LoggingParameters, SingleValue};
use indexmap::IndexMap;

pub fn create_ivy_dockercompose(
    compose_file: PathBuf,
    fluentd_address: &str,
    chain: ethers::types::Chain,
) -> Result<PathBuf, IvyYamlError> {
    let compose_content = std::fs::read_to_string(&compose_file)?;
    let compose = serde_yaml::from_str::<Compose>(&compose_content)?;
    let new_compose = inject_fluentd_logging_driver(compose, fluentd_address, chain);
    let filename = compose_file
        .file_name()
        .ok_or_else(|| IvyYamlError::FilepathError(compose_file.clone()))?
        .to_str()
        .ok_or_else(|| IvyYamlError::FilepathError(compose_file.clone()))?;
    let new_compose_file = compose_file.with_file_name(format!("ivy-{}", filename));
    let new_compose_content = serde_yaml::to_string(&new_compose)?;
    std::fs::write(&new_compose_file, new_compose_content)?;
    Ok(new_compose_file)
}

/// Injects a fluentd logging driver into the compose file. Adds a chain label to each service, as
/// well as a reference to it in the logging driver options.
pub fn inject_fluentd_logging_driver(
    mut compose: Compose,
    fluentd_address: &str,
    chain: ethers::types::Chain,
) -> Compose {
    let mut log_opts = IndexMap::new();

    log_opts
        .insert("fluentd-address".to_string(), SingleValue::String(fluentd_address.to_string()));
    log_opts.insert("tag".to_string(), SingleValue::String("{{.Name}}".to_string()));
    log_opts.insert("labels".to_string(), SingleValue::String("chain".to_string()));

    let logging_driver =
        LoggingParameters { driver: Some("fluentd".to_string()), options: Some(log_opts) };

    let mut chain_label = IndexMap::new();
    chain_label.insert("chain".to_string(), chain.to_string());

    // edit services in place to add log_opts to each service
    for (_, v) in compose.services.0.iter_mut() {
        if let Some(service) = v {
            service.logging = Some(logging_driver.clone());
            service.labels = Labels::Map(chain_label.clone());
        }
    }

    compose
}

#[derive(thiserror::Error, Debug)]
pub enum IvyYamlError {
    #[error(transparent)]
    StdIoError(#[from] std::io::Error),
    #[error(transparent)]
    SerdeYamlError(#[from] serde_yaml::Error),
    #[error("Failed to parse filepath: {0}")]
    FilepathError(PathBuf),
}
