use ethers::types::Address;
use ivynet_grpc::messages::{
    DiskInformation, MachineData, Metrics, NodeDataV2, SignedClientLog, SignedLog,
    SignedMachineData, SignedMetrics, SignedNameChange, SignedNodeDataV2,
};
use ivynet_signer::{
    sign_utils::{
        sign_client_log, sign_log, sign_machine_data, sign_metrics, sign_name_change,
        sign_node_data_v2, IvySigningError,
    },
    IvyWallet,
};
use sysinfo::{Disks, System};
use uuid::Uuid;

mod version_hash {
    include!(concat!(env!("OUT_DIR"), "/version.rs"));
}

use crate::config::IvyConfig;

#[derive(Clone, Debug)]
pub struct IvyMachine {
    pub id: Uuid,
    pub signer: IvyWallet,
}

impl IvyMachine {
    pub fn new(id: Uuid, signer: IvyWallet) -> Self {
        Self { id, signer }
    }

    pub fn from_config(config: &IvyConfig) -> Result<Self, MachineIdentityError> {
        let id = config.machine_id;
        let signer =
            config.identity_wallet().map_err(|_| MachineIdentityError::IdentityWalletError)?;
        Ok(Self { id, signer })
    }

    pub fn system_info(&self) -> SystemInformation {
        SystemInformation::from_system()
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn pubkey(&self) -> Address {
        self.signer.address()
    }

    pub fn sign_metrics(
        &self,
        avs_name: Option<String>,
        metrics: &[Metrics],
    ) -> Result<SignedMetrics, MachineIdentityError> {
        let signature =
            sign_metrics(metrics, &self.signer).map_err(MachineIdentityError::SigningError)?;
        Ok(SignedMetrics {
            machine_id: self.id.into(),
            avs_name,
            metrics: metrics.to_vec(),
            signature: signature.into(),
        })
    }

    pub fn sign_node_data_v2(
        &self,
        node_data: &NodeDataV2,
    ) -> Result<SignedNodeDataV2, MachineIdentityError> {
        let signature = sign_node_data_v2(node_data, &self.signer)
            .map_err(MachineIdentityError::SigningError)?;
        Ok(SignedNodeDataV2 {
            machine_id: self.id.into(),
            node_data: Some(node_data.clone()),
            signature: signature.into(),
        })
    }

    pub fn sign_machine_data(&self) -> Result<SignedMachineData, MachineIdentityError> {
        let version = version_hash::VERSION_HASH;
        let sys_info: SystemInformation = self.system_info();
        let machine_data = MachineData {
            ivynet_version: version.to_string(),
            uptime: sys_info.uptime.to_string(),
            cpu_usage: sys_info.cpu_usage.to_string(),
            cpu_cores: sys_info.cpu_cores.to_string(),
            memory_used: sys_info.memory_usage.to_string(),
            memory_free: sys_info.memory_free.to_string(),
            memory_total: sys_info.memory_total.to_string(),
            disk_used_total: sys_info.disk_total.to_string(),
            disks: sys_info
                .disks
                .iter()
                .map(|d| DiskInformation {
                    id: d.id.clone(),
                    total: d.total.to_string(),
                    free: d.free.to_string(),
                    used: d.used.to_string(),
                })
                .collect(),
        };
        let signature = sign_machine_data(&machine_data, &self.signer)?;
        Ok(SignedMachineData {
            machine_id: self.id.into(),
            signature: signature.into(),
            machine_data: Some(machine_data.clone()),
        })
    }

    pub fn sign_name_change(
        &self,
        old_name: &str,
        new_name: &str,
    ) -> Result<SignedNameChange, MachineIdentityError> {
        let signature = sign_name_change(old_name, new_name, &self.signer)?;
        Ok(SignedNameChange {
            signature: signature.into(),
            machine_id: self.id.into(),
            old_name: old_name.to_string(),
            new_name: new_name.to_string(),
        })
    }

    pub fn sign_log(&self, avs_name: &str, log: &str) -> Result<SignedLog, MachineIdentityError> {
        let signature = sign_log(log, &self.signer).map_err(MachineIdentityError::SigningError)?;
        Ok(SignedLog {
            signature: signature.into(),
            machine_id: self.id.into(),
            avs_name: avs_name.to_string(),
            log: log.to_string(),
        })
    }

    pub fn sign_client_log(&self, log: &str) -> Result<SignedClientLog, MachineIdentityError> {
        let signature =
            sign_client_log(log, &self.signer).map_err(MachineIdentityError::SigningError)?;
        Ok(SignedClientLog {
            signature: signature.into(),
            machine_id: self.id.into(),
            log: log.to_string(),
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MachineIdentityError {
    #[error(transparent)]
    SigningError(#[from] IvySigningError),
    #[error("Failed to load identity wallet")]
    IdentityWalletError,
}

#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub id: String,
    pub total: u64,
    pub free: u64,
    pub used: u64,
}

#[derive(Debug, Clone)]
pub struct SystemInformation {
    pub cpu_cores: u64,
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub memory_free: u64,
    pub memory_total: u64,
    pub disks: Vec<DiskInfo>,
    pub disk_total: u64,
    pub uptime: u64,
}

impl SystemInformation {
    pub fn from_system() -> Self {
        let mut sys: System = System::new();
        sys.refresh_all();

        let cpu_cores = sys.cpus().len() as u64;
        let mut cpu_usage = 0.0;
        for cpu in sys.cpus() {
            cpu_usage += cpu.cpu_usage() as f64;
        }

        let memory_usage = sys.used_memory();
        let memory_free = sys.free_memory();
        let memory_total = sys.total_memory();

        let mut disk_total = 0;

        let mut disks = Vec::new();

        for disk in &Disks::new_with_refreshed_list() {
            if disk.total_space() > 0 {
                let id = format!(
                    "{}:{}",
                    disk.mount_point().to_string_lossy(),
                    disk.name().to_string_lossy()
                );
                disk_total += disk.total_space();

                disks.push(DiskInfo {
                    id,
                    total: disk.total_space(),
                    free: disk.available_space(),
                    used: disk.total_space() - disk.available_space(),
                });
            }
        }

        let uptime = System::uptime();

        Self {
            cpu_cores,
            cpu_usage,
            memory_usage,
            memory_free,
            memory_total,
            disks,
            disk_total,
            uptime,
        }
    }
}

#[cfg(test)]
mod ivy_machine_tests {
    use super::*;
    use ethers::types::Signature;
    use ivynet_signer::sign_utils::{
        recover_log, recover_metrics, recover_name_change, recover_node_data_v2,
    };
    use uuid::Uuid;

    #[tokio::test]
    async fn test_sign_metrics() {
        let id = Uuid::new_v4();
        let wallet = IvyWallet::new();
        let machine = IvyMachine::new(id, wallet);
        let metrics = vec![Metrics::default()];
        let avs_name = Some("test_avs".to_string());
        let signed_metrics =
            machine.sign_metrics(avs_name.clone(), &metrics).expect("sign_metrics should succeed");

        assert_eq!(signed_metrics.machine_id, id.as_bytes());
        assert_eq!(signed_metrics.avs_name, avs_name);
        assert_eq!(signed_metrics.metrics, metrics);
        assert!(!signed_metrics.signature.is_empty());

        let signature = Signature::try_from(signed_metrics.signature.as_slice()).unwrap();
        let recovered = recover_metrics(&metrics, &signature).unwrap();
        assert_eq!(recovered, machine.pubkey());
    }

    #[tokio::test]
    async fn test_sign_node_data_v2() {
        let id = Uuid::new_v4();
        let wallet = IvyWallet::new();
        let machine = IvyMachine::new(id, wallet);
        let node_data = NodeDataV2::default();
        let signed_node_data =
            machine.sign_node_data_v2(&node_data).expect("sign_node_data_v2 should succeed");

        assert_eq!(signed_node_data.machine_id, id.as_bytes());
        assert_eq!(signed_node_data.node_data.unwrap(), node_data);
        assert!(!signed_node_data.signature.is_empty());

        let signature = Signature::try_from(signed_node_data.signature.as_slice()).unwrap();
        let recovered = recover_node_data_v2(&node_data, &signature).unwrap();
        assert_eq!(recovered, machine.pubkey());
    }

    #[tokio::test]
    async fn test_sign_name_change() {
        let id = Uuid::new_v4();
        let wallet = IvyWallet::new();
        let machine = IvyMachine::new(id, wallet);
        let old_name = "old_name";
        let new_name = "new_name";
        let signed_name_change =
            machine.sign_name_change(old_name, new_name).expect("sign_name_change should succeed");

        assert_eq!(signed_name_change.machine_id, id.as_bytes());
        assert_eq!(signed_name_change.old_name, old_name.to_string());
        assert_eq!(signed_name_change.new_name, new_name.to_string());
        assert!(!signed_name_change.signature.is_empty());

        let signature = Signature::try_from(signed_name_change.signature.as_slice()).unwrap();
        let recovered = recover_name_change(old_name, new_name, &signature).unwrap();
        assert_eq!(recovered, machine.pubkey());
    }

    #[tokio::test]
    async fn test_sign_log() {
        let id = Uuid::new_v4();
        let wallet = IvyWallet::new();
        let machine = IvyMachine::new(id, wallet);
        let avs_name = "test_avs";
        let log_message = "Test log message";
        let signed_log = machine.sign_log(avs_name, log_message).expect("sign_log should succeed");

        assert_eq!(signed_log.machine_id, id.as_bytes());
        assert_eq!(signed_log.avs_name, avs_name.to_string());
        assert_eq!(signed_log.log, log_message.to_string());
        assert!(!signed_log.signature.is_empty());

        let signature = Signature::try_from(signed_log.signature.as_slice()).unwrap();
        let recovered = recover_log(log_message, &signature).unwrap();
        assert_eq!(recovered, machine.pubkey());
    }
}
