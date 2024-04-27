use clap::Parser;

use crate::{config, keys};

#[derive(Parser, Debug, Clone)]
pub(crate) enum ConfigCommands {
    #[command(
        name = "import-key",
        about = "Import and save as your default Ethereum private key with a password"
    )]
    ImportPrivateKey {
        private_key: String,
        keyname: Option<String>,
        password: Option<String>,
    },
    #[command(
        name = "create-key",
        about = "Create an Ethereum private key to use with Ivynet and optionally store it with a password"
    )]
    CreatePrivateKey {
        #[arg(long)]
        store: bool,
        keyname: Option<String>,
        password: Option<String>,
        
    },
    #[command(
        name = "get-default-public",
        about = "Get the current default saved keypair's Ethereum address"
    )]
    GetDefaultEthAddress,
    #[command(
        name = "get-default-private",
        about = "Get the current default saved private key"
    )]
    GetDefaultPrivateKey,
    #[command(
        name = "set-rpc",
        about = "Set default URLs to use when connecting to 'mainnet', 'holesky', and 'local' RPC urls"
    )]
    SetRpc { network: String, rpc_url: String },
    #[command(
        name = "get-rpc",
        about = "Get the current default RPC URL for 'mainnet', 'holesky', or 'local'"
    )]
    GetRpc { network: String },
    #[command(
        name = "get-sys-info",
        about = "Get the number of CPU cores, memory, and free disk space on the current machine"
    )]
    GetSysInfo,
}

pub fn parse_config_subcommands(subcmd: ConfigCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        ConfigCommands::ImportPrivateKey {
            private_key,
            keyname,
            password,
        } => keys::import_key(private_key, keyname, password)?,
        ConfigCommands::CreatePrivateKey {
            store,
            keyname,
            password,
        } => keys::create_key(store, keyname, password)?,
        ConfigCommands::SetRpc { network, rpc_url } => config::set_rpc_url(&rpc_url, &network)?,
        ConfigCommands::GetRpc { network } => config::get_rpc_url(&network)?,
        ConfigCommands::GetDefaultEthAddress => println!("Public Key: {}", keys::get_stored_public_key()?),
        ConfigCommands::GetDefaultPrivateKey => {
            let priv_key = hex::encode(keys::WALLET.signer().to_bytes());
            println!("Private key: {:?}", priv_key);
        },
        ConfigCommands::GetSysInfo => {
            let (cpus, mem_info, disk_info) = config::get_system_information()?;
            println!(" --- System Information: --- ");
            println!("CPU Cores: {}", cpus);
            println!("Memory Information:");
            println!("  Total: {}", mem_info);
            println!("Disk Information:");
            println!("  Free: {}", disk_info);
            println!(" --------------------------- ");
        },
    };
    Ok(())
}
