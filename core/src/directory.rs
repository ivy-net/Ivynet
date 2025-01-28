use std::{collections::HashMap, sync::LazyLock};

use ethers::types::{Chain, H160};
use ivynet_macros::h160;

use ivynet_node_type::{
    ActiveSet, AltlayerType, InfiniRouteType, MachType, NodeType, SkateChainType,
};

const ALL_DIRECTORIES: [(Chain, H160); 4] = [
    (Chain::Mainnet, h160!(0x135DDa560e946695d6f155dACaFC6f1F25C1F5AF)),
    (Chain::Mainnet, h160!(0x7133415b33B438843D581013f98A08704316633c)),
    (Chain::Holesky, h160!(0x055733000064333CaDDbC92763c58BF0192fFeBf)),
    (Chain::Holesky, h160!(0x58973d16FFA900D11fC22e5e2B6840d9f7e13401)),
];

const ALL_MAINNET_AVSES: [(NodeType, H160); 42] = [
    (NodeType::EigenDA, h160!(0x870679E138bCdf293b7Ff14dD44b70FC97e12fc0)),
    (NodeType::LagrangeZkWorker, h160!(0x22CAc0e6A1465F043428e8AeF737b3cb09D0eEDa)),
    (NodeType::LagrangeStateCommittee, h160!(0x35F4f28A8d3Ff20EEd10e087e8F96Ea2641E6AA2)),
    (NodeType::EOracle, h160!(0x23221c5bB90C7c57ecc1E75513e2E4257673F0ef)),
    (NodeType::Hyperlane(ActiveSet::Eigenlayer), h160!(0xe8E59c6C8B56F2c178f63BCFC4ce5e5e2359c8fc)),
    (NodeType::K3LabsAvs, h160!(0x83742C346E9f305dcA94e20915aB49A483d33f3E)),
    (NodeType::WitnessChain, h160!(0xD25c2c5802198CB8541987b73A8db4c9BCaE5cC7)),
    (NodeType::AvaProtocol, h160!(0x18343Aa10e3D2F3A861e5649627324aEAD987Adf)),
    (NodeType::Predicate, h160!(0xaCB91045B8bBa06f9026e1A30855B6C4A1c5BaC6)),
    (NodeType::Brevis, h160!(0x9FC952BdCbB7Daca7d420fA55b942405B073A89d)),
    //New
    (
        NodeType::Altlayer(AltlayerType::AltlayerMach),
        h160!(0x71a77037870169d47aad6c2c9360861a4c0df2bf),
    ),
    (NodeType::AltlayerMach(MachType::Xterio), h160!(0x6026b61bdd2252160691cb3f6005b6b72e0ec044)),
    (NodeType::Omni, h160!(0xed2f4d90b073128ae6769a9a8d51547b1df766c8)),
    (NodeType::Automata, h160!(0xe5445838c475a2980e6a88054ff1514230b83aeb)),
    (
        NodeType::AltlayerMach(MachType::DodoChain),
        h160!(0xd50e0931703302b080880c45148f5d83ea66ae2a),
    ),
    (NodeType::OpenLayerMainnet, h160!(0xf7fcff55d5fdaf2c3bbeb140be5e62a2c7d26db3)),
    (NodeType::AltlayerMach(MachType::Cyber), h160!(0x1f2c296448f692af840843d993ffc0546619dcdb)),
    (NodeType::ArpaNetworkNodeClient, h160!(0x1de75eaab2df55d467494a172652579e6fa4540e)),
    (
        NodeType::Altlayer(AltlayerType::GmNetworkMach),
        h160!(0xb3acaf09a1b801e36655b786da4eaa6ae9f5dc37),
    ),
    (NodeType::UnifiAVS, h160!(0x2d86e90ed40a034c753931ee31b1bd5e1970113d)),
    (NodeType::SkateChain(SkateChainType::Base), h160!(0xe008064df9f019d0bff0735fe6887d70b23825ca)),
    (
        NodeType::SkateChain(SkateChainType::Mantle),
        h160!(0xfc569b3b74e15cf48aa684144e072e839fd89380),
    ),
    (NodeType::ChainbaseNetworkV1, h160!(0xb73a87e8f7f9129816d40940ca19dfa396944c71)),
    (NodeType::GoPlusAVS, h160!(0xa3f64d3102a035db35c42a9001bbc83e08c7a366)),
    (
        NodeType::UngateInfiniRoute(InfiniRouteType::Base),
        h160!(0xb3e069fd6dda251acbde09eda547e0ab207016ee),
    ),
    (
        NodeType::UngateInfiniRoute(InfiniRouteType::Polygon),
        h160!(0xf75bc9850f4c44e682537c477c4bb08f71f695da),
    ),
    (
        NodeType::PrimevMevCommit(ActiveSet::Eigenlayer),
        h160!(0xbc77233855e3274e1903771675eb71e602d9dc2e),
    ),
    (NodeType::AlignedLayer, h160!(0xef2a435e5ee44b2041100ef8cbc8ae035166606c)),
    (NodeType::Gasp, h160!(0x9A986296d45C327dAa5998519AE1B3757F1e6Ba1)),
    (NodeType::Bolt(ActiveSet::Symbiotic), h160!(0xA42ec46F2c9DC671a72218E145CC13dc119fB722)),
    (NodeType::Hyperlane(ActiveSet::Symbiotic), h160!(0x59cf937Ea9FA9D7398223E3aA33d92F7f5f986A2)),
    (
        NodeType::DittoNetwork(ActiveSet::Symbiotic),
        h160!(0x8560C667Ae72F28D09465B342A480daB28821f6b),
    ),
    (NodeType::Cycle, h160!(0x759D4335cb712aa188935C2bD3Aa6D205aC61305)),
    (
        NodeType::MishtiNetwork(ActiveSet::Symbiotic),
        h160!(0xe87ff321F5721a9285Ec651d01c0C0B857430c2c),
    ),
    (NodeType::Kalypso, h160!(0x3a7B173124DcFeCff1847FF7f8f56e72ABE02340)),
    (NodeType::RouterXtendNetwork, h160!(0xcf128E88E11507aBAd12a7624A34E3d22F731AbC)),
    (NodeType::CapxCloud, h160!(0xAD12e74847d6D1487A6a3A6b75D1f509f3F627e8)),
    (NodeType::Symbiosis, h160!(0x5112EbA9bc2468Bb5134CBfbEAb9334EdaE7106a)),
    (NodeType::Radius, h160!(0xfCa0128A19A5c06b0148c27ee7623417a11BaAbd)),
    (
        NodeType::PrimevMevCommit(ActiveSet::Symbiotic),
        h160!(0x9101eda106A443A0fA82375936D0D1680D5a64F5),
    ),
    (NodeType::IBTCNetwork, h160!(0xe4661BDbC4f557d2684F8a7C4aF50572e51D4166)),
    (NodeType::ZKLink, h160!(0x213F448e7a1C8DAEDe41cf94883Cc6149244d00F)),
];

const ALL_HOLESKY_AVSES: [(NodeType, H160); 34] = [
    (NodeType::EigenDA, h160!(0xD4A7E1Bd8015057293f0D0A557088c286942e84b)),
    (NodeType::LagrangeStateCommittee, h160!(0x18A74E66cc90F0B1744Da27E72Df338cEa0A542b)),
    (NodeType::LagrangeZkWorker, h160!(0xf98d5de1014110c65c51b85ea55f73863215cc10)),
    (NodeType::EOracle, h160!(0x80FE337623Bc849F4b7379f4AB28aF2b470bEa98)),
    (NodeType::Hyperlane(ActiveSet::Eigenlayer), h160!(0xc76E477437065093D353b7d56c81ff54D167B0Ab)),
    // K3 doesn't seem to have a testnet
    (NodeType::WitnessChain, h160!(0xa987EC494b13b21A8a124F8Ac03c9F530648C87D)),
    (NodeType::AvaProtocol, h160!(0xEA3E82F9Ae371A6a372A6DCffB1a9bD17e0608eF)),
    (NodeType::Predicate, h160!(0x4FC1132230fE16f67531D82ACbB9d78993B23825)),
    (NodeType::Brevis, h160!(0x7A46219950d8a9bf2186549552DA35Bf6fb85b1F)),
    (
        NodeType::Altlayer(AltlayerType::AltlayerMach),
        h160!(0xae9a4497dee2540daf489beddb0706128a99ec63),
    ),
    (NodeType::AltlayerMach(MachType::Xterio), h160!(0x648e5012d7b30755963755f7dd7ff03e2f61bf8b)),
    (NodeType::Omni, h160!(0xa7b2e7830c51728832d33421670dbbe30299fd92)),
    (NodeType::Automata, h160!(0x4665af665df5703445645d243f0fd63ed3b9d132)),
    (NodeType::OpenLayerHolesky, h160!(0xf9b555d1d5be5c24ad9b11a87409d7107a8b6174)),
    (NodeType::AethosHolesky, h160!(0xde93e0da148e1919bb7f33cd8847f96e45791210)),
    (NodeType::UnifiAVS, h160!(0x9b43f227ca57c685a8fe8898eef7dfbd399505df)),
    //The skatechains might be swapped around - unsure
    (NodeType::SkateChain(SkateChainType::Base), h160!(0x5d592a255a4369982aa7fb55c6cbc12c7103e5e4)),
    (
        NodeType::SkateChain(SkateChainType::Mantle),
        h160!(0x32612e4ec0eec067be22bc0d21e26d2cd3322d84),
    ),
    (NodeType::ChainbaseNetworkV1, h160!(0x5e78eff26480a75e06ccdabe88eb522d4d8e1c9d)),
    (NodeType::ChainbaseNetwork, h160!(0x0fb6b02f8482a06cf1d99558576f111abc377932)),
    (NodeType::GoPlusAVS, h160!(0x6e0e0479e177c7f5111682c7025b4412613cd9de)),
    (
        NodeType::UngateInfiniRoute(InfiniRouteType::Base),
        h160!(0x1b8ad2ab0fa5585804ce9e9e2c6097f0328bb05c),
    ),
    (
        NodeType::PrimevMevCommit(ActiveSet::Eigenlayer),
        h160!(0xededb8ed37a43fd399108a44646b85b780d85dd4),
    ),
    (NodeType::AlignedLayer, h160!(0x58f280bebe9b34c9939c3c39e0890c81f163b623)),
    (NodeType::Gasp, h160!(0xb4dd45a08BFA6fBC19F7cD624cdfef87CE95e7AC)),
    (
        NodeType::DittoNetwork(ActiveSet::Eigenlayer),
        h160!(0x5FD0026a449eeA51Bd1471E4ee8df8607aaECC24),
    ),
    (NodeType::Nuffle, h160!(0x2344C0FE02Ccd2b32155Ca0ffcb1978a6d96a552)),
    (NodeType::Blockless, h160!(0x234c91AbD960B72e63d5e63C8246A259f3827Ac8)),
    (NodeType::Primus, h160!(0x3DD26B1e365FBED12B384093FD13e7Ed93fa9979)),
    (NodeType::AtlasNetwork, h160!(0x590dDF9A1a475bF46F10627A49051036d5286a61)),
    (NodeType::Zellular, h160!(0x73746A9a52dD3e925dCE3f4E0f2D69F95755c424)),
    (NodeType::Bolt(ActiveSet::Eigenlayer), h160!(0xa632a3e652110Bb2901D5cE390685E6a9838Ca04)),
    (NodeType::Redstone, h160!(0xBA7A7CaEE3b1ed84a98dBc20Ea20fe21FE7D557e)),
    (
        NodeType::MishtiNetwork(ActiveSet::Eigenlayer),
        h160!(0xe87ff321F5721a9285Ec651d01c0C0B857430c2c),
    ),
];

type AvsMap = HashMap<Chain, HashMap<NodeType, H160>>;

static ALL_AVSES: LazyLock<AvsMap> = LazyLock::new(|| {
    let mut map = HashMap::with_capacity(2);

    let mainnet_avses = ALL_MAINNET_AVSES.into_iter().collect();

    let holesky_avses = ALL_HOLESKY_AVSES.into_iter().collect();

    map.insert(Chain::Mainnet, mainnet_avses);
    map.insert(Chain::Holesky, holesky_avses);
    map
});

static DIRECTORIES_BY_CHAIN: LazyLock<HashMap<Chain, Vec<H160>>> = LazyLock::new(|| {
    ALL_DIRECTORIES.iter().fold(HashMap::with_capacity(2), |mut acc, (chain, addr)| {
        acc.entry(*chain).or_default().push(*addr);
        acc
    })
});

static AVSES_BY_CHAIN: LazyLock<HashMap<Chain, Vec<H160>>> = LazyLock::new(|| {
    let mut map = HashMap::with_capacity(2);

    map.insert(Chain::Mainnet, ALL_MAINNET_AVSES.iter().map(|(_, addr)| *addr).collect());
    map.insert(Chain::Holesky, ALL_HOLESKY_AVSES.iter().map(|(_, addr)| *addr).collect());

    map
});

#[inline]
pub fn get_all_avses() -> &'static AvsMap {
    &ALL_AVSES
}

#[inline]
pub fn get_all_directories_for_chain(chain: Chain) -> Option<&'static Vec<H160>> {
    DIRECTORIES_BY_CHAIN.get(&chain)
}

#[inline]
pub fn get_all_avses_for_chain(chain: Chain) -> Option<&'static Vec<H160>> {
    AVSES_BY_CHAIN.get(&chain)
}

pub fn avs_contract(node_type: NodeType, chain: Chain) -> Option<H160> {
    ALL_AVSES.get(&chain)?.get(&node_type).copied()
}

pub fn get_avs_from_address(address: H160) -> Option<(Chain, NodeType, H160)> {
    // Using Chain iterator if available, otherwise fallback to checking both chains
    [Chain::Mainnet, Chain::Holesky].iter().find_map(|&chain| {
        ALL_AVSES
            .get(&chain)?
            .iter()
            .find_map(|(node_type, &addr)| (addr == address).then_some((chain, *node_type, addr)))
    })
}
