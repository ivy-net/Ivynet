use clap::Subcommand;
use std::fmt::Display;

// TODO: use newtype in ivynet_core to wrap and implement clap subcommands

#[derive(Subcommand, Debug)]
pub enum AvsCommands {
    #[command(name = "info", about = "Get information about the currently running AVS")]
    Info {},
    #[command(name = "setup", about = "opt in to valid quorums with the given AVS")]
    Setup { avs: String, chain: String },
    #[command(name = "optin", about = "opt in to valid quorums with the given AVS")]
    Optin {},
    #[command(name = "optout", about = "opt out of valid quorums with the given AVS")]
    Optout {},
    #[command(name = "start", about = "Start running an AVS node in a docker container")]
    Start {
        #[clap(required(false), long, requires("chain"))]
        avs: Option<String>,
        #[clap(required(false), long, requires("avs"))]
        chain: Option<String>,
    },
    #[command(name = "stop", about = "stop running the active AVS docker container")]
    Stop {},
    #[command(
        name = "setavs",
        about = "unload the current AVS instance and load in a new instance."
    )]
    SetAvs { avs: String, chain: String },
    #[command(
        name = "check-stake-percentage",
        about = "Determine what percentage of the total stake an address would have"
    )]
    CheckStakePercentage { avs: String, address: String, network: String },
}

impl Display for AvsCommands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AvsCommands::Info {} => write!(f, "get information about the currently running AVS"),
            AvsCommands::Setup { avs, chain } => write!(f, "setup {} on chain {}", avs, chain),
            AvsCommands::Optin {} => write!(f, "optin"),
            AvsCommands::Optout {} => write!(f, "optout"),
            AvsCommands::Start { .. } => write!(f, "start"),
            AvsCommands::Stop {} => write!(f, "stop"),
            AvsCommands::CheckStakePercentage { avs, address, network } => {
                write!(f, "check stake percentage for {} on {} network", address, network)?;
                todo!("Use {}", avs)
            }
            AvsCommands::SetAvs { avs, chain } => {
                write!(f, "set AVS to {} on chain {}", avs, chain)
            }
        }
    }
}
