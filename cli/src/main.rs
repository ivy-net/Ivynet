use anyhow::{Context, Error as AnyError, Result};
use clap::{Parser, Subcommand};
use cli::{avs, config, error::Error, init::initialize_ivynet, key, operator, service, staker};
use ivynet_core::{avs::commands::AvsCommands, config::IvyConfig, grpc::client::Uri};
use std::str::FromStr as _;
use tracing_subscriber::FmtSubscriber;

#[allow(unused_imports)]
use tracing::{debug, error, warn, Level};

mod version_hash {
    include!(concat!(env!("OUT_DIR"), "/version.rs"));
}

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

    /// Decide the level of verbosity for the logs
    #[arg(long, env = "LOG_LEVEL", default_value_t = Level::INFO)]
    pub log_level: Level,

    /// Skip backend connection
    #[arg(long, env = "NO_BACKEND", default_value_t = true)]
    pub no_backend: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(name = "version", about = "Return IvyNet version")]
    Version,

    #[command(name = "init", about = "Ivynet config intiliazation")]
    Init,
    #[command(name = "avs", about = "Request information about an AVS or boot up a node")]
    Avs {
        #[command(subcommand)]
        subcmd: AvsCommands,
    },
    #[command(name = "config", about = "Manage rpc and config information")]
    Config {
        #[command(subcommand)]
        subcmd: config::ConfigCommands,
    },
    #[command(name = "key", about = "Manage keys")]
    Key {
        #[command(subcommand)]
        subcmd: key::KeyCommands,
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
        about = "Start the Ivynet service with a specified AVS on CHAIN selected for startup. --avs <AVS> --chain <CHAIN>"
    )]
    Serve {
        #[clap(required(false), long, requires("chain"))]
        avs: Option<String>,
        #[clap(required(false), long, requires("avs"))]
        chain: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let args = Args::parse();

    start_tracing(args.log_level)?;

    // Early return if we're initializing. Init propagates ivyconfig file, and if we attempt to load
    // it before it's been created, this will error.
    if let Commands::Init = args.cmd {
        initialize_ivynet(args.server_url, args.server_ca.as_ref(), args.no_backend).await?;
        return Ok(());
    }

    let config = IvyConfig::load_from_default_path().context("Failed to load ivyconfig. Please ensure `~/.ivynet/ivyconfig.toml` exists and is not malformed. If this is your first time running Ivynet, please run `ivynet init` to perform first-time intialization.")?;

    match args.cmd {
        Commands::Version => {
            println!(
                "ivynet version is {} ({})",
                env!("CARGO_PKG_VERSION"),
                version_hash::VERSION_HASH
            );
        }
        Commands::Config { subcmd } => {
            config::parse_config_subcommands(
                subcmd,
                config,
                args.server_url,
                args.server_ca.as_ref(),
            )
            .await?;
        }
        Commands::Key { subcmd } => key::parse_key_subcommands(subcmd, config).await?,
        Commands::Operator { subcmd } => {
            operator::parse_operator_subcommands(subcmd, &config).await?
        }
        Commands::Staker { subcmd } => staker::parse_staker_subcommands(subcmd, &config).await?,
        Commands::Avs { subcmd } => avs::parse_avs_subcommands(subcmd, &config).await?,
        Commands::Serve { avs, chain } => {
            service::serve(
                avs,
                chain,
                &config,
                args.server_url,
                args.server_ca.as_ref(),
                args.no_backend,
            )
            .await?
        }
        Commands::Init => unreachable!("Init handled above."),
    }

    Ok(())
}

pub fn start_tracing(level: Level) -> Result<(), Error> {
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}
