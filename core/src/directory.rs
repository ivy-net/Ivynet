use std::{collections::HashMap, sync::OnceLock};

use ethers::types::{Chain, H160};
use ivynet_macros::h160;

use crate::node_type::NodeType;

static ALL_DIRECTORIES: &[(Chain, H160)] = &[
    (Chain::Mainnet, h160!(0x135DDa560e946695d6f155dACaFC6f1F25C1F5AF)),
    (Chain::Holesky, h160!(0x055733000064333CaDDbC92763c58BF0192fFeBf)),
];

static ALL_MAINNET_AVSES: &[(Chain, NodeType, H160)] = &[
    (Chain::Mainnet, NodeType::EigenDA, h160!(0x870679E138bCdf293b7Ff14dD44b70FC97e12fc0)),
    (
        Chain::Mainnet,
        NodeType::LagrangeZkWorkerMainnet,
        h160!(0x22CAc0e6A1465F043428e8AeF737b3cb09D0eEDa),
    ),
    (
        Chain::Mainnet,
        NodeType::LagrangeStateCommittee,
        h160!(0x35F4f28A8d3Ff20EEd10e087e8F96Ea2641E6AA2),
    ),
    (Chain::Mainnet, NodeType::EOracle, h160!(0x23221c5bB90C7c57ecc1E75513e2E4257673F0ef)),
    (Chain::Mainnet, NodeType::Hyperlane, h160!(0xe8E59c6C8B56F2c178f63BCFC4ce5e5e2359c8fc)),
    (Chain::Mainnet, NodeType::K3LabsAvs, h160!(0x83742C346E9f305dcA94e20915aB49A483d33f3E)),
    (Chain::Mainnet, NodeType::WitnessChain, h160!(0xD25c2c5802198CB8541987b73A8db4c9BCaE5cC7)),
    (Chain::Mainnet, NodeType::AvaProtocol, h160!(0x18343Aa10e3D2F3A861e5649627324aEAD987Adf)),
    (Chain::Mainnet, NodeType::Predicate, h160!(0xaCB91045B8bBa06f9026e1A30855B6C4A1c5BaC6)),
    (Chain::Mainnet, NodeType::Brevis, h160!(0x9FC952BdCbB7Daca7d420fA55b942405B073A89d)),
    //New
    (Chain::Mainnet, NodeType::AltlayerMach, h160!(0x71a77037870169d47aad6c2c9360861a4c0df2bf)),
    (Chain::Mainnet, NodeType::XterioMACH, h160!(0x6026b61bdd2252160691cb3f6005b6b72e0ec044)),
    (Chain::Mainnet, NodeType::Omni, h160!(0xed2f4d90b073128ae6769a9a8d51547b1df766c8)),
    (Chain::Mainnet, NodeType::Automata, h160!(0xe5445838c475a2980e6a88054ff1514230b83aeb)),
    (Chain::Mainnet, NodeType::DodoChain, h160!(0xd50e0931703302b080880c45148f5d83ea66ae2a)),
    (Chain::Mainnet, NodeType::OpenLayer, h160!(0xf7fcff55d5fdaf2c3bbeb140be5e62a2c7d26db3)),
    (Chain::Mainnet, NodeType::CyberMach, h160!(0x1f2c296448f692af840843d993ffc0546619dcdb)),
    (Chain::Mainnet, NodeType::Aethos, h160!(0x07e26bf8060e33fa3771d88128b75493750515c1)),
    (Chain::Mainnet, NodeType::ArpaNetwork, h160!(0x1de75eaab2df55d467494a172652579e6fa4540e)),
    (Chain::Mainnet, NodeType::OpacityNetwork, h160!(0xce06c5fe42d22ff827a519396583fd9f5176e3d3)),
    (Chain::Mainnet, NodeType::GMNetworkMach, h160!(0xb3acaf09a1b801e36655b786da4eaa6ae9f5dc37)),
    (Chain::Mainnet, NodeType::UnifiAVS, h160!(0x2d86e90ed40a034c753931ee31b1bd5e1970113d)),
    //Which one is right?
    (Chain::Mainnet, NodeType::SkateChainBase, h160!(0xe008064df9f019d0bff0735fe6887d70b23825ca)),
    (Chain::Mainnet, NodeType::SkateChainMantle, h160!(0xfc569b3b74e15cf48aa684144e072e839fd89380)),
    (
        Chain::Mainnet,
        NodeType::ChainbaseNetworkAVS,
        h160!(0xb73a87e8f7f9129816d40940ca19dfa396944c71),
    ),
    (Chain::Mainnet, NodeType::GoPlusAVS, h160!(0xa3f64d3102a035db35c42a9001bbc83e08c7a366)),
    (
        Chain::Mainnet,
        NodeType::UngateInfiniRouteBase,
        h160!(0xb3e069fd6dda251acbde09eda547e0ab207016ee),
    ),
    (
        Chain::Mainnet,
        NodeType::UngateInfiniRoutePolygon,
        h160!(0xf75bc9850f4c44e682537c477c4bb08f71f695da),
    ),
    (Chain::Mainnet, NodeType::PrimevMevCommit, h160!(0xbc77233855e3274e1903771675eb71e602d9dc2e)),
    (Chain::Mainnet, NodeType::AlignedLayer, h160!(0xef2a435e5ee44b2041100ef8cbc8ae035166606c)),
];

static ALL_HOLESKY_AVSES: &[(Chain, NodeType, H160)] = &[
    (Chain::Holesky, NodeType::EigenDA, h160!(0xD4A7E1Bd8015057293f0D0A557088c286942e84b)),
    (
        Chain::Holesky,
        NodeType::LagrangeStateCommittee,
        h160!(0x18A74E66cc90F0B1744Da27E72Df338cEa0A542b),
    ),
    (
        //This might be wrong - no info on etherscan
        Chain::Holesky,
        NodeType::LagrangeZkWorkerHolesky,
        h160!(0xf98d5de1014110c65c51b85ea55f73863215cc10),
    ),
    (Chain::Holesky, NodeType::EOracle, h160!(0x80FE337623Bc849F4b7379f4AB28aF2b470bEa98)),
    (Chain::Holesky, NodeType::Hyperlane, h160!(0xc76E477437065093D353b7d56c81ff54D167B0Ab)),
    // TODO: K3 doesn't seem to have a testnet
    (Chain::Holesky, NodeType::WitnessChain, h160!(0xa987EC494b13b21A8a124F8Ac03c9F530648C87D)),
    (Chain::Holesky, NodeType::AvaProtocol, h160!(0xEA3E82F9Ae371A6a372A6DCffB1a9bD17e0608eF)),
    (Chain::Holesky, NodeType::Predicate, h160!(0x4FC1132230fE16f67531D82ACbB9d78993B23825)),
    (Chain::Holesky, NodeType::Brevis, h160!(0x7A46219950d8a9bf2186549552DA35Bf6fb85b1F)),
    // (
    //     Chain::Holesky,
    //     NodeType::NuffleFastFinality,
    //     h160!(0x2344c0fe02ccd2b32155ca0ffcb1978a6d96a552),
    // ),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
    // (Chain::Holesky, NodeType::BLANKO, h160!(0x00000)),
];

pub fn get_all_avses() -> &'static HashMap<Chain, HashMap<NodeType, H160>> {
    static INSTANCE: OnceLock<HashMap<Chain, HashMap<NodeType, H160>>> = OnceLock::new();
    INSTANCE.get_or_init(|| {
        let mut avses_by_chain: HashMap<Chain, HashMap<NodeType, H160>> = HashMap::new();

        let mut avses = Vec::new();
        avses.extend(ALL_MAINNET_AVSES.iter().copied());
        avses.extend(ALL_HOLESKY_AVSES.iter().copied());

        for (chain, node_type, address) in avses {
            avses_by_chain.entry(chain).or_default().insert(node_type, address);
        }

        avses_by_chain
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
        let mut avses_by_chain = Vec::new();

        if chain == Chain::Mainnet {
            avses_by_chain.extend(ALL_MAINNET_AVSES.iter().map(|(_, _, address)| *address));
        } else if chain == Chain::Holesky {
            avses_by_chain.extend(ALL_HOLESKY_AVSES.iter().map(|(_, _, address)| *address));
        }

        avses_by_chain
    })
}

pub fn avs_contract(node_type: NodeType, chain: Chain) -> Option<H160> {
    if let Some(directories) = get_all_avses().get(&chain) {
        directories.get(&node_type).copied()
    } else {
        None
    }
}

pub fn get_avs_from_address(address: H160) -> Option<(Chain, NodeType, H160)> {
    let mut avses = Vec::new();
    avses.extend(ALL_MAINNET_AVSES.iter().copied());
    avses.extend(ALL_HOLESKY_AVSES.iter().copied());
    avses.iter().find(|(_, _, avs)| *avs == address).copied()
}
