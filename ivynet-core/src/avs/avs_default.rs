use ethers_core::types::Address;
use ethers_signers::Signer;
use std::str::FromStr;

use super::eigenda::eigenda::EigenDA;
use crate::{
    avs::eigenda::eigenda,
    keys::get_wallet,
    rpc_management::{get_network, Network},
};

pub enum AVS {
    EigenDA,
}

//Need to refactor this so its abstract and forces impl on individual avs modules
pub async fn boot_avs(avs: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut avs_dir = dirs::home_dir().expect("Could not get a home directory");
    avs_dir.push(".eigenlayer");

    let network = get_network();
    let operator = get_wallet().address();

    match AVS::from_str(avs) {
        Ok(AVS::EigenDA) => {
            avs_dir.push("eigenda");
            let eigenda = EigenDA::new(avs_dir);
            eigenda.boot(operator, network).await?;
        }
        Err(_) => {
            println!("Invalid AVS: {}", avs);
        }
    }
    Ok(())
}

// TODO: Re-implement check stake and system requirements

impl FromStr for AVS {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "eigenda" => Ok(AVS::EigenDA),
            _ => Err(()),
        }
    }
}
