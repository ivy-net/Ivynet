use std::path::{Path, PathBuf};

use ethers::types::H160;
use once_cell::sync::Lazy;

pub static LOCAL_DEPLOYMENT_DEFAULT_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let workspace_dir = workspace_dir();
    workspace_dir.join("avss/files/eigenlayer/output.json")
});

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Eigenlayer {
    addresses: Addresses,
    chain_info: ChainInfo,
    parameters: Parameters,
}

impl Eigenlayer {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let config = std::fs::read_to_string(path)?;
        let eigenlayer: Eigenlayer = serde_json::from_str(&config)?;
        Ok(eigenlayer)
    }
    // populates env vars from the Eigenlayer struct
    pub fn to_env(&self) {
        std::env::set_var("LOCALHOST_AVS_DIRECTORY", format!("{:?}", self.addresses.avs_directory));
        std::env::set_var(
            "LOCALHOST_DELEGATION_MANAGER",
            format!("{:?}", self.addresses.delegation),
        );
        std::env::set_var(
            "LOCALHOST_EIGEN_POD_MANAGER",
            format!("{:?}", self.addresses.eigen_pod_manager),
        );
        std::env::set_var(
            "LOCALHOST_REWARDS_COORDINATOR",
            format!("{:?}", self.addresses.rewards_coordinator),
        );
        std::env::set_var("LOCALHOST_SLASHER", format!("{:?}", self.addresses.slasher));
        std::env::set_var(
            "LOCALHOST_STRATEGY_MANAGER",
            format!("{:?}", self.addresses.strategy_manager),
        );
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Addresses {
    avs_directory: H160,
    avs_directory_implementation: H160,
    base_strategy_implementation: H160,
    delegation: H160,
    delegation_implementation: H160,
    eigen_layer_pauser_reg: H160,
    eigen_layer_proxy_admin: H160,
    eigen_pod_beacon: H160,
    eigen_pod_implementation: H160,
    eigen_pod_manager: H160,
    eigen_pod_manager_implementation: H160,
    empty_contract: H160,
    rewards_coordinator: H160,
    rewards_coordinator_implementation: H160,
    slasher: H160,
    slasher_implementation: H160,
    // TODO: This probably needs to change, currently unpopulated in output struct
    strategies: String,
    strategy_manager: H160,
    strategy_manager_implementation: H160,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainInfo {
    chain_id: u64,
    deployment_block: u64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameters {
    executor_multisig: H160,
    operations_multisig: H160,
}

// Get workspace root
fn workspace_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

#[test]
#[ignore]
fn test_load_eigenlayer() {
    let _ = Eigenlayer::load(LOCAL_DEPLOYMENT_DEFAULT_PATH.clone()).unwrap();
}

#[test]
#[ignore]
fn test_eigenlayer_to_env() {
    let eigenlayer = Eigenlayer::load(LOCAL_DEPLOYMENT_DEFAULT_PATH.clone()).unwrap();
    eigenlayer.to_env();
    assert_eq!(
        std::env::var("LOCALHOST_AVS_DIRECTORY").unwrap(),
        eigenlayer.addresses.avs_directory.to_string()
    );
    assert_eq!(
        std::env::var("LOCALHOST_DELEGATION_MANAGER").unwrap(),
        eigenlayer.addresses.delegation.to_string()
    );
    assert_eq!(
        std::env::var("LOCALHOST_EIGEN_POD_MANAGER").unwrap(),
        eigenlayer.addresses.eigen_pod_manager.to_string()
    );
    assert_eq!(
        std::env::var("LOCALHOST_REWARDS_COORDINATOR").unwrap(),
        eigenlayer.addresses.rewards_coordinator.to_string()
    );
    assert_eq!(
        std::env::var("LOCALHOST_SLASHER").unwrap(),
        eigenlayer.addresses.slasher.to_string()
    );
    assert_eq!(
        std::env::var("LOCALHOST_STRATEGY_MANAGER").unwrap(),
        eigenlayer.addresses.strategy_manager.to_string()
    );
}
