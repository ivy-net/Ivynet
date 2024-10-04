use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AvsName {
    EigenDA,
    AltLayer,
    LagrangeZK,
    WitnessChain,
    OpenLayer,
}

impl AvsName {
    pub fn as_str(&self) -> &str {
        match self {
            AvsName::EigenDA => "eigenda",
            AvsName::AltLayer => "altlayer",
            AvsName::LagrangeZK => "lagrange",
            AvsName::WitnessChain => "witnesschain",
            AvsName::OpenLayer => "openlayer",
        }
    }
}

impl From<&str> for AvsName {
    fn from(s: &str) -> Self {
        match s {
            "eigenda" => AvsName::EigenDA,
            "altlayer" => AvsName::AltLayer,
            "lagrange" => AvsName::LagrangeZK,
            "witnesschain" => AvsName::WitnessChain,
            "openlayer" => AvsName::OpenLayer,
            _ => panic!("Invalid string for AvsName"),
        }
    }
}
