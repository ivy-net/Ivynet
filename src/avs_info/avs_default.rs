use std::str::FromStr;

use crate::avs_info::eigenda;

pub enum AVS {
    EigenDA,
}

pub async fn boot_avs(avs: &str) -> Result<(), Box<dyn std::error::Error>> {
    match AVS::from_str(&avs) {
        Ok(AVS::EigenDA) => {
            println!("Booting up AVS: EigenDA");
            eigenda::boot_eigenda().await?;
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