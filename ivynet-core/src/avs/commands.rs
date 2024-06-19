use clap::Subcommand;
use std::fmt::Display;

// TODO: use newtype in ivynet_core to wrap and implement clap subcommands

#[derive(Subcommand, Debug)]
pub enum AvsCommands {
    #[command(name = "setup", about = "opt in to valid quorums with the given AVS")]
    Setup { avs: String, chain: String },
    #[command(name = "optin", about = "opt in to valid quorums with the given AVS")]
    Optin { avs: String, chain: String },
    #[command(name = "optout", about = "opt out of valid quorums with the given AVS")]
    Optout { avs: String, chain: String },
    #[command(name = "start", about = "Start running an AVS node in a docker container")]
    Start { avs: String, chain: String },
    #[command(name = "stop", about = "stop running the active AVS docker container")]
    Stop { avs: String, chain: String },
    #[command(
        name = "check-stake-percentage",
        about = "Determine what percentage of the total stake an address would have"
    )]
    CheckStakePercentage { avs: String, address: String, network: String },
}

impl Display for AvsCommands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AvsCommands::Setup { avs, chain } => write!(f, "setup {} on chain {}", avs, chain),
            AvsCommands::Optin { avs, chain } => write!(f, "optin {} on chain {}", avs, chain),
            AvsCommands::Optout { avs, chain } => write!(f, "optout {} on chain {}", avs, chain),
            AvsCommands::Start { avs, chain } => write!(f, "start {} on chain {}", avs, chain),
            AvsCommands::Stop { avs, chain } => write!(f, "stop {} on chain {}", avs, chain),
            AvsCommands::CheckStakePercentage { avs, address, network } => {
                write!(f, "check stake percentage for {} on {} network", address, network)
            }
        }
    }
}
