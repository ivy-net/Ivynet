use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::io::{read_toml, IoError};

/// Types associated with the Lagrange worker config, stored locally in
/// ${LAGRANGE_WORKER_DIR}/worker-conf.toml

/// Config type for the lagrange worker, defined in worker-conf.toml of the Lagrange spec.
#[derive(Debug, Serialize, Deserialize)]
struct LagrangeConfig {
    worker: Worker,
    avs: Avs,
    prometheus: Prometheus,
}

impl LagrangeConfig {
    fn load(path: PathBuf) -> Result<Self, IoError> {
        let config: Self = read_toml(&path)?;
        Ok(config)
    }

    fn store(&self, path: PathBuf) -> Result<(), IoError> {
        crate::io::write_toml(&path, &self)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Worker {
    data_dir: String,
    instance_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Avs {
    gateway_url: String,
    issuer: String,
    worker_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Prometheus {
    port: u16,
}
