use clap::{Parser, Subcommand};
use cli::{avs, config, error::Error, init::initialize_ivynet, operator, service, staker};
use ivynet_core::{avs::commands::AvsCommands, config::IvyConfig, grpc::client::Uri};
use std::str::FromStr as _;
#[allow(unused_imports)]
use tracing::{debug, error, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

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
    #[arg(long, short, default_value = "debug")]
    pub logs: String,
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

    setup_tracing(&args.logs)?;

    match args.cmd {
        Commands::Init => initialize_ivynet()?,
        Commands::Config { subcmd } => {
            config::parse_config_subcommands(
                subcmd,
                check_for_config(),
                args.server_url,
                args.server_ca.as_ref(),
            )
            .await?;
        }
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
            service::serve(avs, chain, &config, &keyfile_pw).await?
        }
    }

    Ok(())
}

// Setup tracing
fn setup_tracing(logs: &str) -> Result<(), Error> {
    let mut filter =
        EnvFilter::builder().parse("ivynet_cli=debug,ivynet_core=debug,tonic=debug")?;

    match logs {
        "trace" => {
            filter =
                EnvFilter::builder().parse("ivynet_cli=trace,ivynet_core=trace,tonic=trace")?;
        }
        "debug" => {
            filter =
                EnvFilter::builder().parse("ivynet_cli=debug,ivynet_core=debug,tonic=debug")?;
        }
        "info" => {
            filter = EnvFilter::builder().parse("ivynet_cli=info,ivynet_core=info,tonic=info")?;
        }
        "warn" => {
            filter = EnvFilter::builder().parse("ivynet_cli=warn,ivynet_core=warn,tonic=warn")?;
        }
        "error" => {
            filter =
                EnvFilter::builder().parse("ivynet_cli=error,ivynet_core=error,tonic=error")?;
        }
        _ => {
            println!("Default log level: DEBUG");
        }
    }
    tracing_subscriber::registry().with(fmt::layer()).with(filter).init();

    Ok(())
}

fn check_for_config() -> IvyConfig {
    IvyConfig::load_from_default_path().unwrap_or_else(|_| {
        panic!("No config file found. Run 'ivynet init' to start initialization.")
    })
}
