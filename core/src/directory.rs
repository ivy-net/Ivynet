use std::{collections::HashMap, sync::OnceLock};

use ethers::types::{Chain, H160};
use ivynet_macros::h160;

use crate::node_type::NodeType;

static ALL_DIRECTORIES: &[(Chain, H160)] = &[
    (Chain::Mainnet, h160!(0x135DDa560e946695d6f155dACaFC6f1F25C1F5AF)),
    (Chain::Holesky, h160!(0x055733000064333CaDDbC92763c58BF0192fFeBf)),
];

static ALL_AVSES: &[(Chain, NodeType, H160)] = &[
    (Chain::Mainnet, NodeType::EigenDA, h160!(0x870679E138bCdf293b7Ff14dD44b70FC97e12fc0)),
    (Chain::Holesky, NodeType::EigenDA, h160!(0xD4A7E1Bd8015057293f0D0A557088c286942e84b)),
    (
        Chain::Mainnet,
        NodeType::LagrangeZkWorkerMainnet,
        h160!(0x35F4f28A8d3Ff20EEd10e087e8F96Ea2641E6AA2),
    ),
    (
        Chain::Holesky,
        NodeType::LagrangeZkWorkerHolesky,
        h160!(0x18A74E66cc90F0B1744Da27E72Df338cEa0A542b),
    ),
    (Chain::Mainnet, NodeType::EOracle, h160!(0x23221c5bB90C7c57ecc1E75513e2E4257673F0ef)),
    (Chain::Holesky, NodeType::EOracle, h160!(0x80FE337623Bc849F4b7379f4AB28aF2b470bEa98)),
    (Chain::Mainnet, NodeType::Hyperlane, h160!(0xe8E59c6C8B56F2c178f63BCFC4ce5e5e2359c8fc)),
    (Chain::Holesky, NodeType::Hyperlane, h160!(0xc76E477437065093D353b7d56c81ff54D167B0Ab)),
    (Chain::Mainnet, NodeType::K3LabsAvs, h160!(0x83742C346E9f305dcA94e20915aB49A483d33f3E)),
    // TODO: K3 doesn't seem to have a testnet
    (Chain::Mainnet, NodeType::WitnessChain, h160!(0xD25c2c5802198CB8541987b73A8db4c9BCaE5cC7)),
    (Chain::Holesky, NodeType::WitnessChain, h160!(0xa987EC494b13b21A8a124F8Ac03c9F530648C87D)),
    (Chain::Mainnet, NodeType::AvaProtocol, h160!(0x18343Aa10e3D2F3A861e5649627324aEAD987Adf)),
    (Chain::Holesky, NodeType::AvaProtocol, h160!(0xEA3E82F9Ae371A6a372A6DCffB1a9bD17e0608eF)),
    (Chain::Mainnet, NodeType::Predicate, h160!(0xaCB91045B8bBa06f9026e1A30855B6C4A1c5BaC6)),
    (Chain::Holesky, NodeType::Predicate, h160!(0x4FC1132230fE16f67531D82ACbB9d78993B23825)),
    (Chain::Mainnet, NodeType::Brevis, h160!(0x9FC952BdCbB7Daca7d420fA55b942405B073A89d)),
    (Chain::Holesky, NodeType::Brevis, h160!(0x7A46219950d8a9bf2186549552DA35Bf6fb85b1F)),
];

pub fn get_all_avses() -> &'static HashMap<Chain, HashMap<NodeType, H160>> {
    static INSTANCE: OnceLock<HashMap<Chain, HashMap<NodeType, H160>>> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        let mut directories_by_chain: HashMap<Chain, HashMap<NodeType, H160>> = HashMap::new();

        for (chain, node_type, address) in ALL_AVSES {
            directories_by_chain.entry(*chain).or_default().insert(*node_type, *address);
        }

        directories_by_chain
    })
}

pub fn get_all_directories_for_chain(chain: Chain) -> &'static Vec<H160> {
    static INSTANCE: OnceLock<Vec<H160>> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        let mut directories_by_chain = Vec::new();

        for (d_chain, address) in ALL_DIRECTORIES {
            if chain == *d_chain {
                directories_by_chain.push(*address)
            }
        }

        directories_by_chain
    })
}

pub fn get_all_avses_for_chain(chain: Chain) -> &'static Vec<H160> {
    static INSTANCE: OnceLock<Vec<H160>> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        let mut directories_by_chain = Vec::new();

        for (d_chain, _, address) in ALL_AVSES {
            if chain == *d_chain {
                directories_by_chain.push(*address)
            }
        }

        directories_by_chain
    })
}

pub fn avs_contract(node_type: NodeType, chain: Chain) -> Option<H160> {
    if let Some(directories) = get_all_avses().get(&chain) {
        directories.get(&node_type).copied()
    } else {
        None
    }
}
