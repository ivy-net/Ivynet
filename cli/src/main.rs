use anyhow::{Error as AnyError, Result};
use clap::{Parser, Subcommand};
use cli::{
    config::{self, IvyConfig},
    error::Error,
    init, key, monitor,
};
use ivynet_grpc::client::Uri;
use std::{fs, path::PathBuf, str::FromStr as _};
use tracing::info;
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

    /// IvyNet servers Uri for communication
    #[arg(long, env = "SERVER_URL", value_parser = Uri::from_str, default_value = if cfg!(debug_assertions) {
        "http://localhost:50050"
    } else {
        "https://api1.test.ivynet.dev:50050"
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
    #[command(name = "monitor", about = "Start node monitor daemon")]
    Monitor,

    #[command(name = "scan", about = "Scanning for existing AVS instances running on the machine")]
    Scan {
        /// For forcing manual container addition even when all other AVS's are already configured
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },

    #[command(
        name = "register-node",
        about = "Register a node with the backend. Requires a correctly configured IvyConfig."
    )]
    RegisterNode,

    #[command(name = "rename-node", about = "Rename a node")]
    RenameNode {
        #[arg(long, short, env = "OLD_NAME")]
        old_name: Option<String>,
        #[arg(long, short, env = "NEW_NAME")]
        new_name: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let args = Args::parse();

    start_tracing(args.log_level)?;

    let config = {
        match IvyConfig::load_from_default_path() {
            Ok(c) => c,
            Err(_) => {
                let mut config = IvyConfig::new();
                config.set_server_url(args.server_url.to_string());
                if let Some(ref ca) = args.server_ca {
                    config.set_server_ca(ca.clone());
                }

                create_config_dir(config.get_path())?;
                config.store()?;

                config
            }
        }
    };

    info!("Parsing commands...");

    match args.cmd {
        Commands::Config { subcmd } => {
            config::parse_config_subcommands(subcmd, config).await?;
        }
        Commands::Key { subcmd } => key::parse_key_subcommands(subcmd).await?,
        // Commands::Node { subcmd } => avs::parse_avs_subcommands(subcmd).await?,
        Commands::Monitor => monitor::start_monitor(config).await?,
        Commands::Scan { force } => monitor::scan(force, &config).await?,
        Commands::RegisterNode => init::register_node(config).await?,
        Commands::RenameNode { old_name, new_name } => {
            monitor::rename_node(&config, old_name, new_name).await?;
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
