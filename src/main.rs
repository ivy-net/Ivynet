use clap::{Parser, Subcommand};
use cli::{avs_cli, config_cli, operator_cli, staker_cli};

mod cli;
mod config;
mod keys;
mod eigen;
mod avs_info;
mod errors;
mod rpc_management;

#[derive(Parser, Debug, Clone)]
#[command(name = "ivy", version, about = "The command line interface for ivynet")]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
    /// The network to connect to: mainnet, testnet, local
    #[arg(long, short, default_value = "local")]
    network: String,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    #[command(
        name = "setup",
        about = "Not implemented yet - First time setup for ivynet! Start here!"
    )]
    Setup {
        #[command(subcommand)]
        subcmd: SetupCommands,
    },
    #[command(name = "config", about = "Manage rpc information, keys, and keyfile settings")]
    Config {
        #[command(subcommand)]
        subcmd: config_cli::ConfigCommands,
    },
    #[command(name = "operator", about = "Request information, register, or manage your operator")]
    Operator {
        #[command(subcommand)]
        subcmd: operator_cli::OperatorCommands,
    },
    #[command(name = "staker", about = "Request information about stakers")]
    Staker {
        #[command(subcommand)]
        subcmd: staker_cli::StakerCommands,
    },
    #[command(
        name = "avs",
        about = "Not implemented yet - Request information about an AVS or boot up a node"
    )]
    Avs {
        #[command(subcommand)]
        subcmd: avs_cli::AvsCommands,
    },
}

#[derive(Parser, Debug, Clone)]
enum SetupCommands {
    #[command(name = "todo", about = "todo")]
    Todo { private_key: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    //TODO: Refactor for subcommands
    rpc_management::set_network(&args.network.clone());
    match args.cmd {
        Commands::Config { subcmd } => config_cli::parse_config_subcommands(subcmd)?,
        Commands::Operator { subcmd } => operator_cli::parse_operator_subcommands(subcmd).await?,
        Commands::Staker { subcmd } => staker_cli::parse_staker_subcommands(subcmd).await?,
        Commands::Setup { subcmd } => match subcmd {
            SetupCommands::Todo { private_key: _ } => todo!(),
        },
        Commands::Avs { subcmd } => avs_cli::parse_config_subcommands(subcmd).await?,
    }

    Ok(())
}
