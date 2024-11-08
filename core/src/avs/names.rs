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

#[derive(Debug, thiserror::Error)]
pub enum AvsParseError {
    #[error("Avs type not found")]
    NotFound,
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

impl TryFrom<&str> for AvsName {
    type Error = AvsParseError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "eigenda" => Ok(AvsName::EigenDA),
            "altlayer" => Ok(AvsName::AltLayer),
            "lagrange" => Ok(AvsName::LagrangeZK),
            "witnesschain" => Ok(AvsName::WitnessChain),
            "openlayer" => Ok(AvsName::OpenLayer),
            _ => Err(AvsParseError::NotFound),
        }
    }
}
