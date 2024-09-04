pub mod delegation_manager;

use ethers::{
    contract::abigen,
    types::{Chain, H160},
};
use ivynet_macros::h160;

pub type AvsDirectory = AvsDirectoryAbi<ethers::providers::Provider<ethers::providers::Http>>;
pub type DelegationManager =
    DelegationManagerAbi<ethers::providers::Provider<ethers::providers::Http>>;
pub type RewardsCoordinator =
    RewardsCoordinatorAbi<ethers::providers::Provider<ethers::providers::Http>>;
pub type Slasher = SlasherAbi<ethers::providers::Provider<ethers::providers::Http>>;
pub type StrategyManager = StrategyManagerAbi<ethers::providers::Provider<ethers::providers::Http>>;

abigen!(
    AvsDirectoryAbi,
    "abi/eigenlayer/AvsDirectory.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    DelegationManagerAbi,
    "abi/eigenlayer/DelegationManager.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    EigenPodManagerAbi,
    "abi/eigenlayer/EigenPodManager.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    RewardsCoordinatorAbi,
    "abi/eigenlayer/RewardsCoordinator.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    SlasherAbi,
    "abi/eigenlayer/Slasher.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

abigen!(
    StrategyManagerAbi,
    "abi/eigenlayer/StrategyManager.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

// Eigenlayer deployment addresses:
// https://github.com/Layr-Labs/eigenlayer-contracts?tab=readme-ov-file#deployments

pub fn avs_directory(chain: ethers::types::Chain) -> H160 {
    match chain {
        Chain::Mainnet => h160!(0x135dda560e946695d6f155dacafc6f1f25c1f5af),
        Chain::Holesky => h160!(0x055733000064333CaDDbC92763c58BF0192fFeBf),
        Chain::AnvilHardhat => {
            let address = std::env::var("LOCALHOST_AVS_DIRECTORY")
                .expect("AVS_DIRECTORY_ADDRESS must be set for localhost testing");
            address.parse().expect("Could not parse LOCALHOST_AVS_DIRECTORY")
        }
        _ => unimplemented!(),
    }
}

pub fn delegation_manager(chain: ethers::types::Chain) -> H160 {
    match chain {
        Chain::Mainnet => h160!(0x39053D51B77DC0d36036Fc1fCc8Cb819df8Ef37A),
        Chain::Holesky => h160!(0xA44151489861Fe9e3055d95adC98FbD462B948e7),
        Chain::AnvilHardhat => {
            let address = std::env::var("LOCALHOST_DELEGATION_MANAGER")
                .expect("LOCALHOST_DELEGATION_MANAGER must be set for localhost testing");
            address.parse().expect("Could not parse LOCALHOST_DELEGATION_MANAGER")
        }
        _ => unimplemented!(),
    }
}

pub fn eigen_pod_manager(chain: ethers::types::Chain) -> H160 {
    match chain {
        Chain::Mainnet => h160!(0x91E677b07F7AF907ec9a428aafA9fc14a0d3A338),
        Chain::Holesky => h160!(0x30770d7E3e71112d7A6b7259542D1f680a70e315),
        Chain::AnvilHardhat => {
            let address = std::env::var("LOCALHOST_EIGEN_POD_MANAGER")
                .expect("LOCALHOST_EIGEN_POD_MANAGER must be set for localhost testing");
            address.parse().expect("Could not parse LOCALHOST_EIGEN_POD_MANAGER")
        }
        _ => unimplemented!(),
    }
}

pub fn rewards_coordinator(chain: ethers::types::Chain) -> H160 {
    match chain {
        Chain::Mainnet => h160!(0x7750d328b314EfFa365A0402CcfD489B80B0adda),
        Chain::Holesky => h160!(0xAcc1fb458a1317E886dB376Fc8141540537E68fE),
        Chain::AnvilHardhat => {
            let address = std::env::var("LOCALHOST_REWARDS_COORDINATOR")
                .expect("LOCALHOST_REWARDS_COORDINATOR must be set for localhost testing");
            address.parse().expect("Could not parse LOCALHOST_REWARDS_COORDINATOR")
        }
        _ => unimplemented!(),
    }
}

pub fn slasher(chain: ethers::types::Chain) -> H160 {
    match chain {
        Chain::Mainnet => h160!(0xD92145c07f8Ed1D392c1B88017934E301CC1c3Cd),
        Chain::Holesky => h160!(0xcAe751b75833ef09627549868A04E32679386e7C),
        Chain::AnvilHardhat => {
            let address = std::env::var("LOCALHOST_SLASHER")
                .expect("LOCALHOST_SLASHER must be set for localhost testing");
            address.parse().expect("Could not parse LOCALHOST_SLASHER")
        }
        _ => unimplemented!(),
    }
}

pub fn strategy_manager(chain: ethers::types::Chain) -> H160 {
    match chain {
        Chain::Mainnet => h160!(0xD92145c07f8Ed1D392c1B88017934E301CC1c3Cd),
        Chain::Holesky => h160!(0xdfB5f6CE42aAA7830E94ECFCcAd411beF4d4D5b6),
        Chain::AnvilHardhat => {
            let address = std::env::var("LOCALHOST_STRATEGY_MANAGER")
                .expect("LOCALHOST_STRATEGY_MANAGER must be set for localhost testing");
            address.parse().expect("Could not parse LOCALHOST_STRATEGY_MANAGER")
        }
        _ => unimplemented!(),
    }
}

#[cfg(test)]
mod tests {
    use crate::test::local_anvil::{fork_holesky_anvil, fork_local_anvil, fork_mainnet_anvil};

    use super::*;
    use ethers::providers::{Middleware, Provider};
    use ethers::types::Chain;

    async fn test_contract_deployed(contract_address: H160, rpc: String) {
        let provider = Provider::try_from(rpc).unwrap();
        let code_bytes = provider.get_code(contract_address, None).await.unwrap();
        assert!(code_bytes.len() > 0);
    }

    mod localhost {
        use super::*;
        #[tokio::test]
        async fn test_localhost_contracts_deployed() {
            // Initialize eigenlayer contracts via env variables
            let local = fork_local_anvil();

            test_contract_deployed(avs_directory(Chain::AnvilHardhat), local.endpoint()).await;
            test_contract_deployed(delegation_manager(Chain::AnvilHardhat), local.endpoint()).await;
            test_contract_deployed(eigen_pod_manager(Chain::AnvilHardhat), local.endpoint()).await;
            test_contract_deployed(rewards_coordinator(Chain::AnvilHardhat), local.endpoint())
                .await;
            test_contract_deployed(slasher(Chain::AnvilHardhat), local.endpoint()).await;
            test_contract_deployed(strategy_manager(Chain::AnvilHardhat), local.endpoint()).await;
        }
    }

    mod holesky {
        use super::*;
        #[tokio::test]
        async fn test_holesky_contracts_deployed() {
            let holesky = fork_holesky_anvil();

            test_contract_deployed(avs_directory(Chain::Holesky), holesky.endpoint()).await;
            test_contract_deployed(delegation_manager(Chain::Holesky), holesky.endpoint()).await;
            test_contract_deployed(eigen_pod_manager(Chain::Holesky), holesky.endpoint()).await;
            test_contract_deployed(rewards_coordinator(Chain::Holesky), holesky.endpoint()).await;
            test_contract_deployed(slasher(Chain::Holesky), holesky.endpoint()).await;
            test_contract_deployed(strategy_manager(Chain::Holesky), holesky.endpoint()).await;
        }
    }

    mod mainnet {
        use super::*;
        #[tokio::test]
        async fn test_mainnet_contracts_deployed() {
            let mainnet = fork_mainnet_anvil();

            test_contract_deployed(avs_directory(Chain::Mainnet), mainnet.endpoint()).await;
            test_contract_deployed(delegation_manager(Chain::Mainnet), mainnet.endpoint()).await;
            test_contract_deployed(eigen_pod_manager(Chain::Mainnet), mainnet.endpoint()).await;
            test_contract_deployed(rewards_coordinator(Chain::Mainnet), mainnet.endpoint()).await;
            test_contract_deployed(slasher(Chain::Mainnet), mainnet.endpoint()).await;
            test_contract_deployed(strategy_manager(Chain::Mainnet), mainnet.endpoint()).await;
        }
    }
}
