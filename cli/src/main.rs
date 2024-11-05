use anyhow::{Error as AnyError, Result};
use clap::{Parser, Subcommand};
use cli::{avs, config, error::Error, key, service};
use ivynet_core::{
    avs::commands::AvsCommands,
    config::IvyConfig,
    fluentd::{make_fluentd_compose, make_fluentd_conf},
    grpc::client::Uri,
};
use std::{fs, path::PathBuf, str::FromStr as _};
use tracing_subscriber::FmtSubscriber;

#[allow(unused_imports)]
use tracing::{debug, error, warn, Level};

mod version_hash {
    include!(concat!(env!("OUT_DIR"), "/version.rs"));
}

#[derive(Parser, Debug)]
#[command(name = "ivy", version = version_hash::VERSION_HASH, about = "The command line interface for ivynet")]
struct Args {
    #[command(subcommand)]
    cmd: Commands,

    /// The network to connect to: mainnet, holesky, local
    #[arg(long, short, default_value = "holesky")]
    network: String,

    /// IvyNet servers Uri for communication
    #[arg(long, env = "SERVER_URL", value_parser = Uri::from_str, default_value = if cfg!(debug_assertions) {
        "http://localhost:50050"
    } else {
        "https://api2.test.ivynet.dev:50050"
    })]
    pub server_url: Uri,

    /// IvyNets server certificate
    #[arg(long, env = "SERVER_CA")]
    pub server_ca: Option<String>,

    /// Decide the level of verbosity for the logs
    #[arg(long, env = "LOG_LEVEL", default_value_t = Level::INFO)]
    pub log_level: Level,
    /// Skip backend connection
    #[arg(long, env = "NO_BACKEND", default_value_t = false)]
    pub no_backend: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
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

    let mut config = {
        match IvyConfig::load_from_default_path() {
            Ok(c) => c,
            Err(_) => {
                let mut config = IvyConfig::new();
                config.set_server_url(args.server_url.to_string());
                if let Some(ref ca) = args.server_ca {
                    config.set_server_ca(ca.clone());
                }

                make_fluentd_compose(config.get_dir());
                make_fluentd_conf(config.get_dir());
                create_config_dir(config.get_path())?;
                config.store()?;

                config
            }
        }
    };

    match args.cmd {
        Commands::Config { subcmd } => {
            config::parse_config_subcommands(subcmd, config).await?;
        }
        Commands::Key { subcmd } => key::parse_key_subcommands(subcmd, config).await?,
        Commands::Avs { subcmd } => avs::parse_avs_subcommands(subcmd, &config).await?,
        Commands::Serve { avs, chain } => {
            service::serve(avs, chain, &mut config, args.no_backend).await?
        }
    }

    Ok(())
}

fn start_tracing(level: Level) -> Result<(), Error> {
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn create_config_dir(config_path: PathBuf) -> Result<(), AnyError> {
    if !config_path.exists() {
        fs::create_dir_all(&config_path)?;
    }
    Ok(())
}
