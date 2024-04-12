use clap::Parser;

mod config;
mod keys;

#[derive(Parser)]
#[command(name = "ivy", version, about = "The command line interface for ivynet")]
pub struct Args {
    /// Import an Eth key and save it
    #[arg(short, long, value_name = "Private Key")]
    import_ecdsa: Option<String>,

    /// Create an Eth key and save it
    #[arg(short, long)]
    create_ecdsa: bool,

    /// Sanity Check to grab restake data from Eth network
    /// using public address
    #[arg(long, value_name = "Public Address")]
    check_restake: Option<String>,

    /// Get the default public EVM address from a local pem file
    #[arg(long)]
    get_address: bool,

    /// Set or update your rpc endpoint url
    #[arg(long, default_value = "http://localhost:8545", value_name = "URL")]
    set_rpc: Option<String>,

    /// Change your default key file located in $HOME/.ivynet/
    #[arg(long, value_name = "path")]
    set_default_keyfile: Option<String>,
}

//TODO: Refacto for subcommands
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

fn main() {
    let args = Args::parse();

    //TODO: Refacto for subcommands
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
        Args { check_restake: Some(_), .. } => println!("TODO"),
        Args { get_address: true, .. } => {
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
        _ => println!("No arguments provided"),
    }
}
