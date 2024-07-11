use async_trait::async_trait;
use ethers::types::{Address, Chain, U256};
use std::{path::PathBuf, sync::Arc};

use super::{eigenda::EigenDA, mach_avs::AltLayer, AvsVariant};
use crate::{config::IvyConfig, eigen::quorum::QuorumType, error::IvyError, rpc_management::IvyProvider};

/// Wrapper type around various AVSes for composition purposes.
/// TODO: Consider alternate nomenclature -- AvsInstance and AvsVariant may not be descriptive
/// enough to prevent ambiguity
pub enum AvsInstance {
    EigenDA(EigenDA),
    AltLayer(AltLayer),
}

// TODO: This should probably be a macro if possible
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AvsVariant for AvsInstance {
    async fn setup(&self, provider: Arc<IvyProvider>, config: &IvyConfig) -> Result<(), IvyError> {
        match self {
            AvsInstance::EigenDA(avs) => avs.setup(provider, config).await?,
            AvsInstance::AltLayer(avs) => avs.setup(provider, config).await?,
        }
        Ok(())
    }
    async fn build_env(&self, provider: Arc<IvyProvider>, config: &IvyConfig) -> Result<(), IvyError> {
        match self {
            AvsInstance::EigenDA(avs) => avs.build_env(provider, config).await?,
            AvsInstance::AltLayer(avs) => avs.build_env(provider, config).await?,
        }
        Ok(())
    }
    fn validate_node_size(&self, quorum_percentage: U256) -> Result<bool, IvyError> {
        match self {
            AvsInstance::EigenDA(avs) => avs.validate_node_size(quorum_percentage),
            AvsInstance::AltLayer(avs) => avs.validate_node_size(quorum_percentage),
        }
    }
    async fn opt_in(
        &self,
        quorums: Vec<QuorumType>,
        eigen_path: PathBuf,
        private_keypath: PathBuf,
        chain: Chain,
    ) -> Result<(), IvyError> {
        match self {
            AvsInstance::EigenDA(avs) => avs.opt_in(quorums, eigen_path, private_keypath, chain).await,
            AvsInstance::AltLayer(avs) => avs.opt_in(quorums, eigen_path, private_keypath, chain).await,
        }
    }
    async fn opt_out(
        &self,
        quorums: Vec<QuorumType>,
        eigen_path: PathBuf,
        private_keypath: PathBuf,
        chain: Chain,
    ) -> Result<(), IvyError> {
        match self {
            AvsInstance::EigenDA(avs) => avs.opt_out(quorums, eigen_path, private_keypath, chain).await,
            AvsInstance::AltLayer(avs) => avs.opt_out(quorums, eigen_path, private_keypath, chain).await,
        }
    }
    async fn start(&self, quorums: Vec<QuorumType>, chain: Chain) -> Result<(), IvyError> {
        match self {
            AvsInstance::EigenDA(avs) => avs.start(quorums, chain).await,
            AvsInstance::AltLayer(avs) => avs.start(quorums, chain).await,
        }
    }
    async fn stop(&self, quorums: Vec<QuorumType>, chain: Chain) -> Result<(), IvyError> {
        match self {
            AvsInstance::EigenDA(avs) => avs.stop(quorums, chain).await,
            AvsInstance::AltLayer(avs) => avs.stop(quorums, chain).await,
        }
    }
    fn quorum_min(&self, chain: Chain, quorum_type: QuorumType) -> U256 {
        match self {
            AvsInstance::EigenDA(avs) => avs.quorum_min(chain, quorum_type),
            AvsInstance::AltLayer(avs) => avs.quorum_min(chain, quorum_type),
        }
    }
    fn quorum_candidates(&self, chain: Chain) -> Vec<QuorumType> {
        match self {
            AvsInstance::EigenDA(avs) => avs.quorum_candidates(chain),
            AvsInstance::AltLayer(avs) => avs.quorum_candidates(chain),
        }
    }
    fn stake_registry(&self, chain: Chain) -> Address {
        match self {
            AvsInstance::EigenDA(avs) => avs.stake_registry(chain),
            AvsInstance::AltLayer(avs) => avs.stake_registry(chain),
        }
    }
    fn registry_coordinator(&self, chain: Chain) -> Address {
        match self {
            AvsInstance::EigenDA(avs) => avs.registry_coordinator(chain),
            AvsInstance::AltLayer(avs) => avs.registry_coordinator(chain),
        }
    }
    fn path(&self) -> PathBuf {
        match self {
            AvsInstance::EigenDA(avs) => avs.path(),
            AvsInstance::AltLayer(avs) => avs.path(),
        }
    }
}
