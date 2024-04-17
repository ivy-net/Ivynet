use clap::{Parser, Subcommand};
use cli::{config_cli, operator_cli, staker_cli};

mod cli;
mod config;
mod keys;
mod rpc;

#[derive(Parser)]
#[command(name = "ivy", version, about = "The command line interface for ivynet")]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
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
        subcmd: AvsCommands,
    },
    #[command(
        name = "network",
        about = "Specify which network to use: mainnet, testnet, or local(default)"
    )]
    Network { network: String },
}

#[derive(Parser, Debug, Clone)]
enum SetupCommands {
    #[command(name = "todo", about = "todo")]
    Todo { private_key: String },
}

#[derive(Parser, Debug, Clone)]
enum AvsCommands {
    #[command(name = "todo", about = "todo")]
    Todo { private_key: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    //TODO: Refactor for subcommands
    match args.cmd {
        Commands::Network { network } => {
            println!("Network: {:?}", network);
        }
        Commands::Config { subcmd } => config_cli::parse_config_subcommands(subcmd)?,
        Commands::Operator { subcmd } => operator_cli::parse_operator_subcommands(subcmd).await?,
        Commands::Staker { subcmd } => staker_cli::parse_staker_subcommands(subcmd).await?,
        Commands::Setup { subcmd } => match subcmd {
            SetupCommands::Todo { private_key } => todo!(),
        },
        Commands::Avs { subcmd } => match subcmd {
            AvsCommands::Todo { private_key } => todo!(),
        },
    }

    Ok(())
}
