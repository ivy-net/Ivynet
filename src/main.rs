use clap::Parser;
use eyre::Result;

mod config;
mod keys;
mod rpc;

#[derive(Parser)]
#[command(name = "ivy", version, about = "The command line interface for ivynet")]
pub struct Args {
    /// Import an Eth key and save it - start here
    #[arg(short, long, value_name = "Private Key")]
    import_ecdsa: Option<String>,

    /// Create an Eth key and save it - or start here
    #[arg(short, long)]
    create_ecdsa: bool,

    /// Sanity Check to grab restake data from Eth network using public address
    #[arg(long, value_name = "Public Address")]
    check_operator_stake: Option<String>,

    /// Get the default public EVM address from a local pem file
    #[arg(long)]
    get_stored_address: bool,

    /// Set or update your rpc endpoint url
    #[arg(long, value_name = "URL")]
    set_rpc: Option<String>,
    //Default values in this struct are ALWAYS executed, so until I figure out how to
    //stop execution even when we already have values, we cannot have default values
    //Need to read more about how clap works
    /// View saved RPC url
    #[arg(long)]
    get_rpc: bool,

    /// Change your default key file located in $HOME/.ivynet/
    #[arg(long, value_name = "path")]
    set_default_keyfile: Option<String>,
}

//TODO: Refactor for subcommands
// struct Args {
//     #[command(subcommand)]
//     cmd: Commands
// }

// #[derive(Subcommand, Debug, Clone)]
// enum Commands {
//     // #[command(name = "get", about = "Get a value")]
//     // Get {
//     //     #[arg(about = "The key to get")]
//     //     key: String,
//     // },
//     // #[command(name = "set", about = "Set a value")]
//     // Set {
//     //     #[arg(about = "The key to set")]
//     //     key: String,
//     //     #[arg(about = "The value to set")]
//     //     value: String,
//     // },
//     #[command(name = "import_private_key", about = "Import your private key to ivynet")]
//     ImportPrivateKey(private_key: String),
// }

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    //TODO: Refactor for subcommands
    // match args.cmd {
    //     Commands::ImportPrivateKey { private_key } => keys::key_setup(private_key),
    // }

    //Ugly
    match args {
        Args {
            import_ecdsa: Some(private_key),
            ..
        } => keys::key_setup(private_key),
        Args { create_ecdsa: true, .. } => keys::key_setup("".to_string()),
        Args {
            check_operator_stake: Some(address),
            ..
        } => {
            println!("Checking restake data for address: {}", address);
            // println!("Block number: {:?}", rpc::reads::get_block().await);
            // println!("OperatorDetails: {:?}", rpc::reads::get_operator_details(address).await);
            println!("Staker strategy details for address: {:?}", address);
            rpc::reads::get_staker_delegatable_shares(address).await;
        }
        Args {
            get_stored_address: true, ..
        } => {
            let config = config::load_config();
            let keyfile = config.default_keyfile;
            println!("Keyfile: {}", keyfile);

            let addr = keys::get_eth_address_from_secret(keys::open_pem(keyfile));
            println!("Address: 0x{}", addr);
        }
        Args {
            set_rpc: Some(rpc_string), ..
        } => config::set_rpc_url(rpc_string),
        Args {
            set_default_keyfile: Some(path),
            ..
        } => config::set_default_keyfile(path),
        Args { get_rpc: true, .. } => {
            let config = config::load_config();
            println!("RPC URL: {}", config.rpc_url);
        }
        _ => println!("No arguments provided"),
    }

    Ok(())
}
