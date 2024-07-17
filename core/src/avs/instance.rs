use async_trait::async_trait;
use ethers::types::{Address, Chain, U256};
use std::{path::PathBuf, process::Child, sync::Arc};

use super::{eigenda::EigenDA, lagrange::Lagrange, mach_avs::AltLayer, AvsVariant};
use crate::{
    config::IvyConfig, eigen::quorum::QuorumType, error::IvyError, rpc_management::IvyProvider,
};

/// Wrapper type around various AVSes for composition purposes.
/// TODO: Consider alternate nomenclature -- AvsInstance and AvsVariant may not be descriptive
/// enough to prevent ambiguity
#[derive(Debug, Clone)]
pub enum AvsType {
    EigenDA(EigenDA),
    AltLayer(AltLayer),
    Lagrange(Lagrange),
}

impl AvsType {
    pub fn name(&self) -> &str {
        match self {
            AvsType::EigenDA(_) => "EigenDA",
            AvsType::AltLayer(_) => "AltLayer",
            AvsType::Lagrange(_) => "Lagrange",
        }
    }

    pub fn new(id: &str, chain: Chain) -> Result<Self, IvyError> {
        match id.to_ascii_lowercase().as_str() {
            "eigenda" => Ok(AvsType::EigenDA(EigenDA::new_from_chain(chain))),
            "altlayer" => Ok(AvsType::AltLayer(AltLayer::default())), // TODO: Altlayer init
            "lagrange" => Ok(AvsType::Lagrange(Lagrange::default())),
            _ => Err(IvyError::InvalidAvsType(id.to_string())),
        }
    }
}

// TODO: This should probably be a macro if possible
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AvsVariant for AvsType {
    async fn setup(&self, provider: Arc<IvyProvider>, config: &IvyConfig) -> Result<(), IvyError> {
        match self {
            AvsType::EigenDA(avs) => avs.setup(provider, config).await?,
            AvsType::AltLayer(avs) => avs.setup(provider, config).await?,
            AvsType::Lagrange(avs) => avs.setup(provider, config).await?,
        }
        Ok(())
    }
    async fn build_env(
        &self,
        provider: Arc<IvyProvider>,
        config: &IvyConfig,
    ) -> Result<(), IvyError> {
        match self {
            AvsType::EigenDA(avs) => avs.build_env(provider, config).await?,
            AvsType::AltLayer(avs) => avs.build_env(provider, config).await?,
            AvsType::Lagrange(avs) => avs.build_env(provider, config).await?,
        }
        Ok(())
    }
    fn validate_node_size(&self, quorum_percentage: U256) -> Result<bool, IvyError> {
        match self {
            AvsType::EigenDA(avs) => avs.validate_node_size(quorum_percentage),
            AvsType::AltLayer(avs) => avs.validate_node_size(quorum_percentage),
            AvsType::Lagrange(avs) => avs.validate_node_size(quorum_percentage),
        }
    }
    // TODO: Deprecate
    // async fn opt_in(
    //     &self,
    //     quorums: Vec<QuorumType>,
    //     eigen_path: PathBuf,
    //     private_keypath: PathBuf,
    //     keyfile_password: &str,
    //     chain: Chain,
    // ) -> Result<(), IvyError> {
    //     match self {
    //         AvsType::EigenDA(avs) => {
    //             avs.opt_in(quorums, eigen_path, private_keypath, keyfile_password, chain).await
    //         }
    //         AvsType::AltLayer(avs) => {
    //             avs.opt_in(quorums, eigen_path, private_keypath, keyfile_password, chain).await
    //         }
    //         AvsType::Lagrange(avs) => {
    //             avs.opt_in(quorums, eigen_path, private_keypath, keyfile_password, chain).await
    //         }
    //     }
    // }
    // async fn opt_out(
    //     &self,
    //     quorums: Vec<QuorumType>,
    //     eigen_path: PathBuf,
    //     private_keypath: PathBuf,
    //     keyfile_password: &str,
    //     chain: Chain,
    // ) -> Result<(), IvyError> {
    //     match self {
    //         AvsType::EigenDA(avs) => {
    //             avs.opt_out(quorums, eigen_path, private_keypath, keyfile_password, chain).await
    //         }
    //         AvsType::AltLayer(avs) => {
    //             avs.opt_out(quorums, eigen_path, private_keypath, keyfile_password, chain).await
    //         }
    //         AvsType::Lagrange(avs) => {
    //             avs.opt_out(quorums, eigen_path, private_keypath, keyfile_password, chain).await
    //         }
    //     }
    // }
    async fn start(&mut self, quorums: Vec<QuorumType>, chain: Chain) -> Result<Child, IvyError> {
        match self {
            AvsType::EigenDA(avs) => avs.start(quorums, chain).await,
            AvsType::AltLayer(avs) => avs.start(quorums, chain).await,
            AvsType::Lagrange(avs) => avs.start(quorums, chain).await,
        }
    }
    async fn stop(&mut self, chain: Chain) -> Result<(), IvyError> {
        match self {
            AvsType::EigenDA(avs) => avs.stop(chain).await,
            AvsType::AltLayer(avs) => avs.stop(chain).await,
            AvsType::Lagrange(avs) => avs.stop(chain).await,
        }
    }
    fn quorum_min(&self, chain: Chain, quorum_type: QuorumType) -> U256 {
        match self {
            AvsType::EigenDA(avs) => avs.quorum_min(chain, quorum_type),
            AvsType::AltLayer(avs) => avs.quorum_min(chain, quorum_type),
            AvsType::Lagrange(avs) => avs.quorum_min(chain, quorum_type),
        }
    }
    fn quorum_candidates(&self, chain: Chain) -> Vec<QuorumType> {
        match self {
            AvsType::EigenDA(avs) => avs.quorum_candidates(chain),
            AvsType::AltLayer(avs) => avs.quorum_candidates(chain),
            AvsType::Lagrange(avs) => avs.quorum_candidates(chain),
        }
    }
    fn stake_registry(&self, chain: Chain) -> Address {
        match self {
            AvsType::EigenDA(avs) => avs.stake_registry(chain),
            AvsType::AltLayer(avs) => avs.stake_registry(chain),
            AvsType::Lagrange(avs) => avs.stake_registry(chain),
        }
    }
    fn registry_coordinator(&self, chain: Chain) -> Address {
        match self {
            AvsType::EigenDA(avs) => avs.registry_coordinator(chain),
            AvsType::AltLayer(avs) => avs.registry_coordinator(chain),
            AvsType::Lagrange(avs) => avs.registry_coordinator(chain),
        }
    }
    fn path(&self) -> PathBuf {
        match self {
            AvsType::EigenDA(avs) => avs.path(),
            AvsType::AltLayer(avs) => avs.path(),
            AvsType::Lagrange(avs) => avs.path(),
        }
    }

    fn running(&self) -> bool {
        match self {
            AvsType::EigenDA(avs) => avs.running(),
            AvsType::AltLayer(avs) => avs.running(),
            AvsType::Lagrange(avs) => avs.running(),
        }
    }
}
