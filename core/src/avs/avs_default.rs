use std::{str::FromStr, sync::Arc};

use super::{eigenda::EigenDA, instance::AvsInstance, mach_avs::AltLayer, AvsProvider};
use ethers::{providers::Middleware, types::Chain};

use crate::{config::IvyConfig, error::IvyError, rpc_management::connect_provider, wallet::IvyWallet};

enum AVS {
    EigenDA,
    AltLayer,
}

pub async fn opt_in(avs: &str, chain: Chain, config: &IvyConfig, wallet: Option<IvyWallet>) -> Result<(), IvyError> {
    let mut avs_dir = dirs::home_dir().expect("Could not get a home directory");
    avs_dir.push(".eigenlayer");

    let provider = connect_provider(&config.get_rpc_url(chain)?, wallet).await?;
    let chain: Chain = Chain::try_from(provider.get_chainid().await?).unwrap_or_default();

    let avs = match AVS::from_str(avs) {
        Ok(AVS::EigenDA) => AvsInstance::EigenDA(EigenDA::default()),
        Ok(AVS::AltLayer) => AvsInstance::AltLayer(AltLayer::default()),
        Err(_) => return Err(IvyError::AvsParseError),
    };
    let avs_provider = AvsProvider::new(chain, avs, Arc::new(provider));
    avs_provider.opt_in(config).await?;
    Ok(())
}

pub async fn opt_out(avs: &str, chain: Chain, config: &IvyConfig, wallet: Option<IvyWallet>) -> Result<(), IvyError> {
    let mut avs_dir = dirs::home_dir().expect("Could not get a home directory");
    avs_dir.push(".eigenlayer");

    let provider = connect_provider(&config.get_rpc_url(chain)?, wallet).await?;
    let chain: Chain = Chain::try_from(provider.get_chainid().await?).unwrap_or_default();

    let avs = match AVS::from_str(avs) {
        Ok(AVS::EigenDA) => AvsInstance::EigenDA(EigenDA::default()),
        Ok(AVS::AltLayer) => AvsInstance::AltLayer(AltLayer::default()),
        Err(_) => return Err(IvyError::AvsParseError),
    };
    let avs_provider = AvsProvider::new(chain, avs, Arc::new(provider));

    avs_provider.opt_out(config).await?;
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
