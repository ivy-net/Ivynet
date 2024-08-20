use clap::{Parser, Subcommand};
use cli::{avs, config, error::Error, init::initialize_ivynet, key, operator, service, staker};
use ivynet_core::{avs::commands::AvsCommands, config::IvyConfig, grpc::client::Uri};
use std::str::FromStr as _;
use tracing_subscriber::FmtSubscriber;

#[allow(unused_imports)]
use tracing::{debug, error, warn, Level};

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
async fn main() -> Result<(), Error> {
    let args = Args::parse();

    start_tracing(args.log_level)?;

    match args.cmd {
        Commands::Init => {
            initialize_ivynet(args.server_url, args.server_ca.as_ref(), args.no_backend).await?
        }
        Commands::Config { subcmd } => {
            config::parse_config_subcommands(
                subcmd,
                check_for_config(),
                args.server_url,
                args.server_ca.as_ref(),
            )
            .await?;
        }
        Commands::Key { subcmd } => key::parse_key_subcommands(subcmd, check_for_config()).await?,
        Commands::Operator { subcmd } => {
            operator::parse_operator_subcommands(subcmd, &check_for_config()).await?
        }
        Commands::Staker { subcmd } => {
            staker::parse_staker_subcommands(subcmd, &check_for_config()).await?
        }
        Commands::Avs { subcmd } => avs::parse_avs_subcommands(subcmd, &check_for_config()).await?,
        Commands::Serve { avs, chain } => {
            let config = check_for_config();
            let keyfile_pw = dialoguer::Password::new()
                .with_prompt("Input the password for your stored Operator ECDSA keyfile")
                .interact()?;
            service::serve(
                avs,
                chain,
                &config,
                &keyfile_pw,
                args.server_url,
                args.server_ca.as_ref(),
                args.no_backend,
            )
            .await?
        }
    }

    Ok(())
}

pub fn start_tracing(level: Level) -> Result<(), Error> {
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn check_for_config() -> IvyConfig {
    IvyConfig::load_from_default_path().unwrap_or_else(|_| {
        panic!("No config file found. Run 'ivynet init' to start initialization.")
    })
}
