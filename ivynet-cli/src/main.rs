use clap::{Parser, Subcommand};
use ethers::types::Chain;
use ivynet_core::config::IvyConfig;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use ivynet_cli::{avs, config, error::Error, operator, staker};

#[derive(Parser, Debug, Clone)]
#[command(name = "ivy", version, about = "The command line interface for ivynet")]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
    /// The network to connect to: mainnet, holesky, local
    #[arg(long, short, default_value = "local")]
    network: String,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    #[command(name = "setup", about = "Not implemented yet - First time setup for ivynet! Start here!")]
    Setup {
        #[command(subcommand)]
        subcmd: SetupCommands,
    },
    #[command(name = "config", about = "Manage rpc information, keys, and keyfile settings")]
    Config {
        #[command(subcommand)]
        subcmd: config::ConfigCommands,
    },
    #[command(name = "operator", about = "Request information, register, or manage your operator")]
    Operator {
        #[command(subcommand)]
        subcmd: operator::OperatorCommands,
    },
    #[command(name = "staker", about = "Request information about stakers")]
    Staker {
        #[command(subcommand)]
        subcmd: staker::StakerCommands,
    },
    #[command(name = "avs", about = "Not implemented yet - Request information about an AVS or boot up a node")]
    Avs {
        #[command(subcommand)]
        subcmd: avs::AvsCommands,
    },
}

#[derive(Parser, Debug, Clone)]
enum SetupCommands {
    #[command(name = "todo", about = "todo")]
    Todo { private_key: String },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();

    // Set up tracing
    let filter = EnvFilter::builder().parse("ivynet_cli=debug,ivynet_core=debug")?;
    tracing_subscriber::registry().with(fmt::layer()).with(filter).init();

    let mut config = IvyConfig::load();
    let chain = args.network.parse::<Chain>().expect("Unknown network");

    match args.cmd {
        Commands::Config { subcmd } => {
            config::parse_config_subcommands(subcmd, &mut config, chain)?;
            config.store()?;
        }
        Commands::Operator { subcmd } => operator::parse_operator_subcommands(subcmd, &config, chain).await?,
        Commands::Staker { subcmd } => staker::parse_staker_subcommands(subcmd, &config, chain).await?,
        Commands::Setup { subcmd } => match subcmd {
            SetupCommands::Todo { private_key: _ } => todo!(),
        },
        Commands::Avs { subcmd } => avs::parse_config_subcommands(subcmd, &config, chain).await?,
    }

    Ok(())
}
