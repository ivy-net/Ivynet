use clap::Subcommand;
use std::fmt::Display;

use super::config::NodeType;

#[derive(Subcommand, Debug)]
pub enum RegisterCommands {
    #[command(
        name = "optin",
        about = "Opt in to valid quorums with the given AVS. Valid for: EigenDA, AltLayer"
    )]
    Optin {},
    #[command(
        name = "optout",
        about = "Opt out of valid quorums with the given AVS. Valid for: EigenDA, AltLayer"
    )]
    Optout {},
    #[command(name = "register", about = "Register an operator. Valid for: Lagrange")]
    Register {},
}

#[derive(Subcommand, Debug)]
pub enum NodeCommands {
    #[command(name = "info", about = "Get information about the currently running AVS")]
    Info {},
    #[command(name = "configure", about = "Configure a new node instance.")]
    Configure { node_type: NodeType },
    #[command(
        name = "setup",
        about = "Setup a new AVS instance or enter path information to attach to an existing AVS."
    )]
    Register {},
    #[command(
        name = "unregister",
        about = "Unregister an operator for the loaded AVS. Not valid for all AVS types. See AVS specific dcoumentation for details."
    )]
    Unregister {},
    #[command(
        name = "start",
        about = "Start running an AVS node in a docker container based on a configuration file."
    )]
    Start {},
    #[command(name = "stop", about = "Stop running the active AVS docker container.")]
    Stop {},
    #[command(name = "attach", about = "Attach a running AVS node to a docker container.")]
    Attach {
        #[clap(required(false), long, requires("chain"))]
        avs: Option<String>,
        #[clap(required(false), long, requires("avs"))]
        chain: Option<String>,
    },
    #[command(
        name = "inspect",
        about = "Inspect logs from a given AVS. Defaults to currently selected AVS and chain if not provided."
    )]
    Inspect {
        #[clap(required(false), long, requires("chain"))]
        avs: Option<String>,
        #[clap(required(false), long, requires("avs"))]
        chain: Option<String>,
    },
}

impl Display for NodeCommands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeCommands::Info {} => write!(f, "get information about the currently running AVS"),
            NodeCommands::Configure { node_type } => {
                write!(f, "Configure a new {node_type} node instance")
            }
            NodeCommands::Register {} => write!(f, "register"),
            NodeCommands::Unregister {} => write!(f, "unregister"),
            NodeCommands::Start { .. } => write!(f, "start"),
            NodeCommands::Stop {} => write!(f, "stop"),
            NodeCommands::Attach { .. } => {
                write!(f, "Attaching to active AVS")
            }
            NodeCommands::Inspect { avs: _, chain: _ } => {
                write!(f, "inspect logs")
            }
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum LogCommands {
    #[command(name = "stdout", about = "get all stdout logs")]
    STDOUT,
    #[command(name = "stderr", about = "get all stderr logs")]
    STDERR,
    #[command(name = "debug", about = "get debug logs from stdout")]
    DEBUG,
    #[command(name = "info", about = "get info logs from stdout")]
    INFO,
    #[command(name = "warn", about = "get warning logs from stdout")]
    WARN,
    #[command(name = "error", about = "get error logs from stdout")]
    ERROR,
}

impl Display for LogCommands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogCommands::STDOUT => write!(f, "stdout"),
            LogCommands::STDERR => write!(f, "stderr"),
            LogCommands::DEBUG => write!(f, "debug"),
            LogCommands::INFO => write!(f, "info"),
            LogCommands::WARN => write!(f, "warn"),
            LogCommands::ERROR => write!(f, "error"),
        }
    }
}
