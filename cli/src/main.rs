use anyhow::{Context, Error as AnyError, Result};
use clap::{Parser, Subcommand};
use cli::{avs, config, error::Error, init::initialize_ivynet, key, service};
use ivynet_core::{
    avs::commands::AvsCommands,
    config::IvyConfig,
    grpc::{
        backend::backend_client::BackendClient,
        client::{create_channel, Request, Source, Uri},
        messages::RegistrationCredentials,
    },
};
use std::str::FromStr as _;
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
    #[arg(long, env = "SERVER_URL", value_parser = Uri::from_str, default_value = "https://api1.test.ivynet.dev:50050")]
    pub server_url: Uri,

    /// IvyNets server certificate
    #[arg(long, env = "SERVER_CA")]
    pub server_ca: Option<String>,

    /// Decide the level of verbosity for the logs
    #[arg(long, env = "LOG_LEVEL", default_value_t = Level::INFO)]
    pub log_level: Level,
    // /// Skip backend connection
    // #[arg(long, env = "NO_BACKEND", default_value_t = false)]
    // pub no_backend: bool,
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
    // #[command(name = "operator", about = "Request information, register, or manage your
    // operator")] Operator {
    //     #[command(subcommand)]
    //     subcmd: operator::OperatorCommands,
    // },
    // #[command(name = "staker", about = "Request information about stakers")]
    // Staker {
    //     #[command(subcommand)]
    //     subcmd: staker::StakerCommands,
    // },
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
    #[command(
        name = "register",
        about = "Register this node on IvyNet server using IvyNet details"
    )]
    Register {
        /// Email address registered at IvyNet portal
        #[arg(long, env = "IVYNET_EMAIL")]
        email: String,

        /// Password to IvyNet account
        #[arg(long, env = "IVYNET_PASSWORD")]
        password: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let args = Args::parse();

    start_tracing(args.log_level)?;

    // Early return if we're initializing. Init propagates ivyconfig file, and if we attempt to load
    // it before it's been created, this will error.
    if let Commands::Init = args.cmd {
        initialize_ivynet(args.server_url, args.server_ca, false).await?;
        return Ok(());
    }

    let config = IvyConfig::load_from_default_path().context("Failed to load ivyconfig. Please ensure `~/.ivynet/ivyconfig.toml` exists and is not malformed. If this is your first time running Ivynet, please run `ivynet init` to perform first-time intialization.")?;

    let (server_url, server_ca) = {
        match (config.get_server_url(), config.get_server_ca()) {
            (Ok(uri), ca) => (
                if !uri.to_string().is_empty() { uri } else { args.server_url },
                if !ca.is_empty() { Some(ca) } else { args.server_ca },
            ),
            (Err(_), _) => (args.server_url, args.server_ca),
        }
    };
    match args.cmd {
        Commands::Config { subcmd } => {
            config::parse_config_subcommands(subcmd, config).await?;
        }
        Commands::Key { subcmd } => key::parse_key_subcommands(subcmd, config).await?,
        // Commands::Operator { subcmd } => {
        //     operator::parse_operator_subcommands(subcmd, &config).await?
        // }
        // Commands::Staker { subcmd } => staker::parse_staker_subcommands(subcmd, &config).await?,
        Commands::Avs { subcmd } => avs::parse_avs_subcommands(subcmd, &config).await?,
        Commands::Serve { avs, chain } => {
            service::serve(avs, chain, &config, server_url, server_ca, false).await?
        }
        Commands::Register { email, password } => {
            let config = IvyConfig::load_from_default_path()?;
            let public_key = config.identity_wallet()?.address();
            let hostname =
                { String::from_utf8(rustix::system::uname().nodename().to_bytes().to_vec()) }?;

            let mut backend =
                BackendClient::new(create_channel(Source::Uri(server_url), server_ca).await?);
            backend
                .register(Request::new(RegistrationCredentials {
                    email,
                    password,
                    hostname,
                    public_key: public_key.as_bytes().to_vec(),
                }))
                .await?;
            println!("Node registered");
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
