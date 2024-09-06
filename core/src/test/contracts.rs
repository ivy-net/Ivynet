use ethers::{
    contract::abigen,
    types::{Chain, H160},
};
use ivynet_macros::h160;

abigen!(
    AnkrStethAbi,
    "abi/ankr-steth/AnkrSteth.json",
    event_derives(serde::Deserialize, serde::Serialize)
);

pub fn ankr_staked_eth(chain: ethers::types::Chain) -> H160 {
    match chain {
        Chain::Mainnet => h160!(0x84db6eE82b7Cf3b47E8F19270abdE5718B936670),
        Chain::Holesky => h160!(0xb2f5B45Aa301fD478CcffC93DBD2b91C22FDeDae),
        _ => unimplemented!(),
    }
}

abigen!(ERC20, "abi/token/ERC20.json", event_derives(serde::Deserialize, serde::Serialize));
