use std::str::FromStr;

use crate::{avs_info::eigenda, rpc_management::Network};

pub enum AVS {
    EigenDA,
}

//Need to refactor this so its abstract and forces impl on individual avs modules
pub async fn boot_avs(avs: &str) -> Result<(), Box<dyn std::error::Error>> {
    match AVS::from_str(&avs) {
        Ok(AVS::EigenDA) => {
            eigenda::boot_eigenda().await?;
        }
        Err(_) => {
            println!("Invalid AVS: {}", avs);
        }
    }
    Ok(())
}

pub async fn check_stake_and_system_requirements(
    avs: &str,
    address: &str,
    network: Network,
) -> Result<(), Box<dyn std::error::Error>> {
    match AVS::from_str(&avs) {
        Ok(AVS::EigenDA) => {
            eigenda::check_stake_and_system_requirements(address, network).await?;
        }
        Err(_) => {
            println!("Invalid AVS: {}", avs);
        }
    }
    Ok(())
}

impl FromStr for AVS {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "eigenda" => Ok(AVS::EigenDA),
            _ => Err(()),
        }
    }
}

impl AVS {
    pub fn to_string(&self) -> String {
        match self {
            AVS::EigenDA => "EigenDA".to_string(),
        }
    }
}
