use clap::{Parser, Subcommand};
use ivynet_core::{avs::commands::AvsCommands, config::IvyConfig, grpc::client::Uri};
use std::str::FromStr as _;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[allow(unused_imports)]
use tracing::{debug, error, warn};

use ivynet_cli::{avs, config, error::Error, init::initialize_ivynet, operator, service, staker};

#[derive(Parser, Debug)]
#[command(name = "ivy", version, about = "The command line interface for ivynet")]
struct Args {
    #[command(subcommand)]
    cmd: Commands,

    /// The network to connect to: mainnet, holesky, local
    #[arg(long, short, default_value = "holesky")]
    network: String,

    /// IvyNet servers Uri for communication
    #[arg(long, env = "SERVER_URL", value_parser = Uri::from_str, default_value = "http://localhost:50050")]
    pub server_url: Uri,

    /// IvyNets server certificate
    #[arg(long, env = "SERVER_CA")]
    pub server_ca: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(name = "init", about = "Ivynet config intiliazation")]
    Init,
    #[command(name = "avs", about = "Request information about an AVS or boot up a node")]
    Avs {
        #[command(subcommand)]
        subcmd: AvsCommands,
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
    #[command(
        name = "serve",
        about = "Start the Ivynet service with a specified AVS on a specified chain. --avs <AVS> --chain <CHAIN> [PORT]"
    )]
    Serve {
        #[clap(required(false), long, requires("chain"))]
        avs: Option<String>,
        #[clap(required(false), long, requires("avs"))]
        chain: Option<String>,
        #[clap(required(false), long)]
        port: Option<u16>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();

    // Set up tracing
    let filter = EnvFilter::builder().parse("ivynet_cli=debug,ivynet_core=debug,tonic=debug")?;
    tracing_subscriber::registry().with(fmt::layer()).with(filter).init();

    let config = IvyConfig::load_from_default_path()?;
    match args.cmd {
        Commands::Init => initialize_ivynet()?,
        Commands::Config { subcmd } => {
            config::parse_config_subcommands(subcmd, config, args.server_url, args.server_ca.as_ref()).await?;
        }
        Commands::Operator { subcmd } => operator::parse_operator_subcommands(subcmd, &config).await?,
        Commands::Staker { subcmd } => staker::parse_staker_subcommands(subcmd, &config).await?,
        Commands::Avs { subcmd } => avs::parse_avs_subcommands(subcmd, &config).await?,
        Commands::Serve { avs, chain, port } => service::serve(avs, chain, port, &config).await?,
    }

    Ok(())
}
