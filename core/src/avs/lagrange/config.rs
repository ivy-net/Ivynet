use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::io::{read_toml, IoError};

/// Types associated with the Lagrange worker config, stored locally in
/// ${LAGRANGE_WORKER_DIR}/worker-conf.toml

/// Config type for the lagrange worker, defined in worker-conf.toml of the Lagrange spec.
#[derive(Debug, Serialize, Deserialize)]
pub struct LagrangeConfig {
    pub worker: Worker,
    pub avs: Avs,
    pub prometheus: Prometheus,
}

impl LagrangeConfig {
    pub fn load(path: PathBuf) -> Result<Self, IoError> {
        let config: Self = read_toml(&path)?;
        Ok(config)
    }

    pub fn store(&self, path: PathBuf) -> Result<(), IoError> {
        crate::io::write_toml(&path, &self)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Worker {
    data_dir: String,
    instance_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Avs {
    gateway_url: String,
    issuer: String,
    pub worker_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Prometheus {
    port: u16,
}
