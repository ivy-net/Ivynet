pub enum AvsNames {
    EigenDA,
    AltLayer,
    LagrangeZK,
    WitnessChain,
    OpenLayer,
}

impl AvsNames {
    pub fn as_str(&self) -> &str {
        match self {
            AvsNames::EigenDA => "eigenda",
            AvsNames::AltLayer => "altlayer",
            AvsNames::LagrangeZK => "lagrange",
            AvsNames::WitnessChain => "witnesschain",
            AvsNames::OpenLayer => "openlayer",
        }
    }
}

impl From<&str> for AvsNames {
    fn from(s: &str) -> Self {
        match s {
            "eigenda" => AvsNames::EigenDA,
            "altlayer" => AvsNames::AltLayer,
            "lagrange" => AvsNames::LagrangeZK,
            "witnesschain" => AvsNames::WitnessChain,
            "openlayer" => AvsNames::OpenLayer,
            _ => panic!("Invalid string for AvsNames"),
        }
    }
}
