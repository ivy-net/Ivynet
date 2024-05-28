use std::str::FromStr;

use super::{eigenda::EigenDA, mach_avs::AltLayer, AvsProvider};
use crate::rpc_management::{get_client, get_network, get_signer, Network};

pub enum AVS {
    EigenDA,
    AltLayer,
}

//Need to refactor this so its abstract and forces impl on individual avs modules
pub async fn boot_avs(avs: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut avs_dir = dirs::home_dir().expect("Could not get a home directory");
    avs_dir.push(".eigenlayer");

    let network = get_network();
    let provider = get_client();
    let signer = get_signer();

    let mut avs_dir = match network {
        Network::Mainnet => avs_dir.join("mainnet"),
        Network::Holesky => avs_dir.join("holesky"),
        Network::Local => todo!("Unimplemented"),
    };

    match AVS::from_str(avs) {
        Ok(AVS::EigenDA) => {
            avs_dir.push("eigenda");
            let avs = EigenDA::default();
            let avs_provider = AvsProvider::new(network, avs, provider, signer, avs_dir);
            avs_provider.boot(network).await?;
        }
        Ok(AVS::AltLayer) => {
            avs_dir.push("altlayer");
            let avs = AltLayer::default();
            let avs_provider = AvsProvider::new(network, avs, provider, signer, avs_dir);
            avs_provider.boot(network).await?;
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
