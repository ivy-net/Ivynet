use std::path::PathBuf;

use docker_compose_types::{Compose, LoggingParameters, SingleValue};
use indexmap::IndexMap;

#[allow(dead_code)]
pub fn create_ivy_dockercompose(
    compose_file: PathBuf,
    fluentd_address: &str,
) -> Result<PathBuf, IvyYamlError> {
    let compose_content = std::fs::read_to_string(&compose_file)?;
    let compose = serde_yaml::from_str::<Compose>(&compose_content)?;
    // Template code preserved for translation from fluentd driver to custom driver
    let new_compose = inject_fluentd_logging_driver(compose, fluentd_address);
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

#[allow(dead_code)]
pub fn inject_fluentd_logging_driver(mut compose: Compose, fluentd_address: &str) -> Compose {
    let mut log_opts = IndexMap::new();
    log_opts
        .insert("fluentd-address".to_string(), SingleValue::String(fluentd_address.to_string()));
    log_opts.insert("tag".to_string(), SingleValue::String("{{.Name}}".to_string()));
    let logging_driver =
        LoggingParameters { driver: Some("fluentd".to_string()), options: Some(log_opts) };
    // edit services in plcae to add log_opts to each service
    for (_, v) in compose.services.0.iter_mut() {
        if let Some(service) = v {
            service.logging = Some(logging_driver.clone());
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

// TODO: Currently fails due to changing fluentd to custom logging driver.
#[test]
#[ignore]
fn test_inject_logging_driver() {
    let compose_string = r#"
version: '3.8'
services:
  reverse-proxy:
    image: nginx:latest
    ports:
      - "${NODE_RETRIEVAL_PORT}:${NODE_RETRIEVAL_PORT}"
      - "${NODE_API_PORT}:${NODE_API_PORT}"
    volumes:
      - "${NODE_NGINX_CONF_HOST}:/etc/nginx/templates/default.conf.template:ro"
    depends_on:
      - da-node
    networks:
      - eigenda
    environment:
      - "REQUEST_LIMIT=10r/s"
      - "NODE_HOST=${MAIN_SERVICE_NAME}"
      - "BURST_LIMIT=50"
    env_file:
      - .env
    restart: unless-stopped
  da-node:
    env_file:
      - .env
    container_name: ${MAIN_SERVICE_NAME}
    image: ${MAIN_SERVICE_IMAGE}
    ports:
      - "${NODE_DISPERSAL_PORT}:${NODE_DISPERSAL_PORT}"
      - "${NODE_METRICS_PORT}:${NODE_METRICS_PORT}"
    networks:
      - eigenda
    volumes:
#      Uncomment the following line if you want Node to use ecdsa key. This is generally not recommended
#      - "${NODE_ECDSA_KEY_FILE_HOST}:/app/operator_keys/ecdsa_key.json:readonly"
      - "${NODE_BLS_KEY_FILE_HOST}:/app/operator_keys/bls_key.json:readonly"
      - "${NODE_G1_PATH_HOST}:/app/g1.point:readonly"
      - "${NODE_G2_PATH_HOST}:/app/g2.point.powerOf2:readonly"
      - "${NODE_CACHE_PATH_HOST}:/app/cache:rw"
      - "${NODE_LOG_PATH_HOST}:/app/logs:rw"
      - "${NODE_DB_PATH_HOST}:/data/operator/db:rw"
    restart: unless-stopped
networks:
  eigenda:
    name: ${NETWORK_NAME}
"#;
    let mut compose_content = match serde_yaml::from_str::<Compose>(compose_string) {
        Ok(c) => c,
        Err(e) => panic!("Failed to parse docker-compose file: {}", e),
    };
    let mut log_opts = IndexMap::new();
    log_opts
        .insert("fluentd-address".to_string(), SingleValue::String("localhost:24224".to_string()));
    let logging_driver =
        LoggingParameters { driver: Some("fluentd".to_string()), options: Some(log_opts) };
    // edit services in plcae to add log_opts to each service
    for (k, v) in compose_content.services.0.iter_mut() {
        if let Some(service) = v {
            service.logging = Some(logging_driver.clone());
        }
    }
    todo!()
}
