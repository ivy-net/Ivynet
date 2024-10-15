use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
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

impl fmt::Display for AvsName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
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

impl From<&String> for AvsName {
    fn from(s: &String) -> Self {
        Self::from(s.as_str())
    }
}
