use clap::{Parser, Subcommand};
use ethers_core::types::Address;
use ethers::types::Address;

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
        subcmd: ConfigCommands,
    },
    #[command(name = "operator", about = "Request information, register, or manage your operator")]
    Operator {
        #[command(subcommand)]
        subcmd: OperatorCommands,
    },
    #[command(name = "staker", about = "Request information about stakers")]
    Staker {
        #[command(subcommand)]
        subcmd: StakerCommands,
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

// pub struct Args {
//     /// Get the default public EVM address from a local pem file
//     #[arg(long)]
//     get_stored_address: bool,

//     /// Set or update your rpc endpoint url
//     #[arg(long, value_name = "URL")]
//     set_rpc: Option<String>,
//     //Default values in this struct are ALWAYS executed, so until I figure out how to
//     //stop execution even when we already have values, we cannot have default values
//     //Need to read more about how clap works
//     /// View saved RPC url
//     #[arg(long)]
//     get_rpc: bool,

//     /// Change your default key file located in $HOME/.ivynet/
//     #[arg(long, value_name = "path")]
//     set_default_keyfile: Option<String>,

//     /// Check to grab restake data from Eth network using public address
//     #[arg(long, value_name = "Public Address")]
//     check_operator_stake: Option<String>,

//     /// Sanity check to grab saved address's delegated stake data
//     #[arg(long, value_name = "Public Address")]
//     check_my_stake: Option<String>,

//     /// Get operator details
//     #[arg(long, value_name = "Public Address")]
//     get_operator_details: Option<String>,

// }

#[derive(Parser, Debug, Clone)]
enum ConfigCommands {
    #[command(
        name = "import-private-key",
        about = "Import and save as your default Ethereum private key - WARNING: Not production ready - not encrypted!"
    )]
    ImportPrivateKey {
        #[arg(long)]
        private_key: String,
        #[arg(long)]
        keyname: Option<String>,
    },
    #[command(
        name = "create-key",
        about = "Create an Ethereum private key to use with Ivynet - WARNING: Not production ready - not encrypted!"
    )]
    CreatePrivateKey {
        #[arg(long)]
        store_as_default: bool,
        #[arg(long)]
        keyname: Option<String>,
    },
    #[command(
        name = "get-default-eth-address",
        about = "Get the current default saved keypair's Ethereum address"
    )]
    GetDefaultEthAddress,
    #[command(
        name = "get-default-private-key",
        about = "Get the current default saved private key - WARNING: Not production ready - not encrypted!"
    )]
    GetDefaultPrivateKey,
    #[command(
        name = "set-rpc",
        about = "Set default URLs to use when connecting to 'mainnet', 'testnet', and 'local' RPC urls"
    )]
    SetRpc {
        #[arg(long)]
        network: String,
        #[arg(long)]
        rpc_url: String,
    },
    #[command(
        name = "get-rpc",
        about = "Get the current default RPC URL for 'mainnet', 'testnet', or 'local'"
    )]
    GetRpc {
        #[arg(long)]
        network: String,
    },
}

#[derive(Parser, Debug, Clone)]
enum OperatorCommands {
    #[command(name = "get_operator_details", about = "Get operator details")]
    GetOperatorDetails {
        #[arg(long)]
        address: String,
    },
    #[command(
        name = "get_operator_stake",
        about = "Get an operator's total delineated stake per strategy"
    )]
    GetOperatorStake {
        #[arg(long)]
        address: String,
    },
}

#[derive(Parser, Debug, Clone)]
enum StakerCommands {
    #[command(
        name = "get-staker-shares",
        about = "Get data on a staker's strategy choices and their stake in each one"
    )]
    GetStakerShares {
        #[arg(long)]
        private_key: String,
    },
    #[command(
        name = "get-my-shares",
        about = "Get data on the saved keypair's current strategy and stake"
    )]
    GetMyShares,
}

#[derive(Parser, Debug, Clone)]
enum SetupCommands {
    #[command(name = "todo", about = "todo")]
    Todo {
        #[arg(long)]
        private_key: String,
    },
}

#[derive(Parser, Debug, Clone)]
enum AvsCommands {
    #[command(name = "todo", about = "todo")]
    Todo {
        #[arg(long)]
        private_key: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    //TODO: Refactor for subcommands
    match args.cmd {
        Commands::Network { network } => {
            println!("Network: {:?}", network);
        }
        Commands::Config { subcmd } => match subcmd {
            ConfigCommands::ImportPrivateKey { private_key, keyname } => keys::import_key(private_key, keyname),
            ConfigCommands::CreatePrivateKey {
                store_as_default,
                keyname,
            } => keys::create_key(store_as_default, keyname),
            ConfigCommands::SetRpc { network, rpc_url } => config::set_rpc_url(rpc_url, network)?,
            ConfigCommands::GetRpc { network } => config::get_rpc_url(network)?,
            ConfigCommands::GetDefaultEthAddress => {
                let addr: Address = keys::connect_wallet();
                println!("Info: {:?}", addr);
            }
            ConfigCommands::GetDefaultPrivateKey => {
                let key = keys::get_keystring();
                println!("Private key: {:?}", key);
            }
        },
        Commands::Operator { subcmd } => match subcmd {
            OperatorCommands::GetOperatorDetails { address } => {
                rpc::delegation_manager::get_operator_details(address).await?
            }
            OperatorCommands::GetOperatorStake { address } => todo!(),
        },
        Commands::Staker { subcmd } => match subcmd {
            StakerCommands::GetStakerShares { private_key } => todo!(),
            StakerCommands::GetMyShares => todo!(),
        },
        Commands::Setup { subcmd } => match subcmd {
            SetupCommands::Todo { private_key } => todo!(),
        },
        Commands::Avs { subcmd } => match subcmd {
            AvsCommands::Todo { private_key } => todo!(),
        },
    }

    //Ugly
    // match args {
    //     Args {
    //         import_ecdsa: Some(private_key),
    //         ..
    //     } => keys::key_setup(private_key),
    //     Args { create_ecdsa: true, .. } => keys::key_setup("".to_string()),

    //     Args {
    //         get_stored_address: true, ..
    //     } => {
    //         let config = config::load_config();
    //         let keyfile = config.default_keyfile;
    //         println!("Keyfile: {}", keyfile);

    //         let addr = keys::get_eth_address_from_secret(keys::open_pem(keyfile));
    //         println!("Address: 0x{}", addr);
    //     }
    //     Args {
    //         set_rpc: Some(rpc_string), ..
    //     } => config::set_rpc_url(rpc_string),
    //     Args {
    //         set_default_keyfile: Some(path),
    //         ..
    //     } => config::set_default_keyfile(path),
    //     Args {
    //         check_operator_stake: Some(address),
    //         ..
    //     } => {
    //         rpc::reads::get_all_statregies_delegated_stake(address).await?;
    //         // println!("Checking restake data for address: {}", address);
    //         // println!("Staker strategy details for address: {:?}", address);
    //         // rpc::reads::get_staker_delegatable_shares(address).await?;
    //     }
    //     Args {
    //         get_operator_details: Some(address),
    //         ..
    //     } => {
    //         rpc::reads::get_operator_details(address).await?;
    //     }
    //     Args { get_rpc: true, .. } => {
    //         let config = config::load_config();
    //         println!("RPC URL: {}", config.rpc_url);
    //     }
    //     _ => println!("No arguments provided"),
    // }

    Ok(())
}
