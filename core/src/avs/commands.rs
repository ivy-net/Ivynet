use clap::Subcommand;
use std::fmt::Display;

#[derive(Subcommand, Debug)]
pub enum RegisterCommands {
    #[command(
        name = "optin",
        about = "opt in to valid quorums with the given AVS. Valid for: EigenDA, AltLayer"
    )]
    Optin {},
    #[command(
        name = "optout",
        about = "opt out of valid quorums with the given AVS. Valid for: EigenDA, AltLayer"
    )]
    Optout {},
    #[command(name = "register", about = "register an operator. Valid for: Lagrange")]
    Register {},
}

#[derive(Subcommand, Debug)]
pub enum AvsCommands {
    #[command(name = "info", about = "Get information about the currently running AVS")]
    Info {},
    #[command(name = "setup", about = "opt in to valid quorums with the given AVS")]
    Setup { avs: String, chain: String },
    #[command(
        name = "register",
        about = "Register an operator for the loaded AVS. Not valid for all AVS types. See AVS specific dcoumentation for details."
    )]
    Register {},
    #[command(
        name = "unregister",
        about = "Unregister an operator for the loaded AVS. Not valid for all AVS types. See AVS specific dcoumentation for details."
    )]
    Unregister {},
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
        name = "select",
        about = "unload the current AVS instance and load in a new instance."
    )]
    Select { avs: String, chain: String },
    #[command(name = "attach", about = "attach a running AVS node to a docker container")]
    Attach {
        #[clap(required(false), long, requires("chain"))]
        avs: Option<String>,
        #[clap(required(false), long, requires("avs"))]
        chain: Option<String>,
    },
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
            AvsCommands::Register {} => write!(f, "register"),
            AvsCommands::Unregister {} => write!(f, "unregister"),
            AvsCommands::Start { .. } => write!(f, "start"),
            AvsCommands::Stop {} => write!(f, "stop"),
            AvsCommands::Attach { .. } => {
                write!(f, "Attaching to active AVS")
            }
            AvsCommands::CheckStakePercentage { avs, address, network } => {
                write!(f, "check stake percentage for {} on {} network", address, network)?;
                todo!("Use {}", avs)
            }
            AvsCommands::Select { avs, chain } => {
                write!(f, "set AVS to {} on chain {}", avs, chain)
            }
        }
    }
}
