use std::str::FromStr as _;

use clap::{Parser, Subcommand};
use ivynet_core::{
    config::IvyConfig,
    ethers::types::Chain,
    grpc::{
        backend::backend_client::BackendClient,
        client::{create_channel, Request, Uri},
        messages::RegistrationCredentials,
    },
};
use tracing::{debug, error, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use ivynet_cli::{avs, config, error::Error, init::initialize_ivynet, operator, staker};

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
    #[command(name = "setup", about = "Not implemented yet - First time setup for ivynet! Start here!")]
    Setup {
        #[command(subcommand)]
        subcmd: SetupCommands,
    },
    #[command(name = "init", about = "Ivynet config intiliazation")]
    Init,
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
    #[command(name = "avs", about = "Request information about an AVS or boot up a node")]
    Avs {
        #[command(subcommand)]
        subcmd: avs::AvsCommands,
    },
}

#[derive(Parser, Debug, Clone)]
enum SetupCommands {
    #[command(name = "todo", about = "todo")]
    Todo { private_key: String },

    #[command(name = "register", about = "Register node on IvyNet server")]
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
async fn main() -> Result<(), Error> {
    let args = Args::parse();

    // Set up tracing
    let filter = EnvFilter::builder().parse("ivynet_cli=debug,ivynet_core=debug")?;
    tracing_subscriber::registry().with(fmt::layer()).with(filter).init();

    match args.cmd {
        Commands::Init {} => initialize_ivynet()?,
        Commands::Config { subcmd } => {
            let mut config = IvyConfig::load_from_default_path()?;
            config::parse_config_subcommands(subcmd, &mut config)?;
            config.store()?;
        }
        Commands::Operator { subcmd } => {
            let config = IvyConfig::load_from_default_path()?;
            operator::parse_operator_subcommands(subcmd, &config).await?
        }
        Commands::Staker { subcmd } => {
            let config = IvyConfig::load_from_default_path()?;
            staker::parse_staker_subcommands(subcmd, &config).await?
        }
        Commands::Setup { subcmd } => match subcmd {
            SetupCommands::Todo { private_key: _ } => todo!(),
            SetupCommands::Register { email, password } => {
                let config = IvyConfig::load_from_default_path()?;
                let public_key = config.identity_wallet()?.address();
                let mut backend = BackendClient::new(create_channel(&args.server_url, args.server_ca.as_ref()));
                backend
                    .register(Request::new(RegistrationCredentials {
                        email,
                        password,
                        public_key: public_key.as_bytes().to_vec(),
                    }))
                    .await?;
                println!("Node registered");
            }
        },
        Commands::Avs { subcmd } => {
            let config = IvyConfig::load_from_default_path()?;
            avs::parse_avs_subcommands(subcmd, &config).await?
        }
    }

    Ok(())
}
