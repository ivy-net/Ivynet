use std::{str::FromStr, sync::Arc};

use super::{eigenda::EigenDA, mach_avs::AltLayer, AvsProvider};
use ethers::{providers::Middleware, types::Chain};

use crate::{config::IvyConfig, error::IvyError, rpc_management::connect_provider, wallet::IvyWallet};

pub enum AVS {
    EigenDA,
    AltLayer,
}

//Need to refactor this so its abstract and forces impl on individual avs modules
pub async fn boot_avs(avs: &str, chain: Chain, config: &IvyConfig, wallet: Option<IvyWallet>) -> Result<(), IvyError> {
    let mut avs_dir = dirs::home_dir().expect("Could not get a home directory");
    avs_dir.push(".eigenlayer");

    let provider = connect_provider(&config.get_rpc_url(chain)?, wallet)?;

    let chain: Chain = Chain::try_from(provider.get_chainid().await?).unwrap_or_default();
    let avs_dir = match chain {
        Chain::Mainnet => avs_dir.join("mainnet"),
        Chain::Holesky => avs_dir.join("holesky"),
        _ => todo!("Unimplemented"),
    };

    match AVS::from_str(avs) {
        Ok(AVS::EigenDA) => {
            let avs = EigenDA::default();
            let avs_provider = AvsProvider::new(chain, avs, Arc::new(provider), avs_dir.join("eigenda"));
            avs_provider.boot(config).await?;
        }
        Ok(AVS::AltLayer) => {
            let avs = AltLayer::default();
            let avs_provider = AvsProvider::new(chain, avs, Arc::new(provider), avs_dir.join("altlayer"));
            avs_provider.boot(config).await?;
        }
        Err(_) => {
            println!("Invalid AVS: {}", avs);
        }
    }
    Ok(())
}

// TODO: Re-implement check stake

impl FromStr for AVS {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "eigenda" => Ok(AVS::EigenDA),
            "altlayer" => Ok(AVS::AltLayer),
            _ => Err(()),
        }
    }
}
