use clap::Parser;

use crate::{config, keys};

#[derive(Parser, Debug, Clone)]
pub(crate) enum ConfigCommands {
    #[command(
        name = "import-private-key",
        about = "Import and save as your default Ethereum private key - WARNING: Not production ready - not encrypted!"
    )]
    ImportPrivateKey {
        private_key: String,
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
    SetRpc { network: String, rpc_url: String },
    #[command(
        name = "get-rpc",
        about = "Get the current default RPC URL for 'mainnet', 'testnet', or 'local'"
    )]
    GetRpc { network: String },
}

pub fn parse_config_subcommands(subcmd: ConfigCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        ConfigCommands::ImportPrivateKey { private_key, keyname } => keys::import_key(private_key, keyname),
        ConfigCommands::CreatePrivateKey {
            store_as_default,
            keyname,
        } => keys::create_key(store_as_default, keyname),
        ConfigCommands::SetRpc { network, rpc_url } => config::set_rpc_url(rpc_url, network)?,
        ConfigCommands::GetRpc { network } => config::get_rpc_url(network)?,
        ConfigCommands::GetDefaultEthAddress => {
            println!("Info: {:?}", keys::connect_wallet());
        }
        ConfigCommands::GetDefaultPrivateKey => {
            let key = keys::get_keystring();
            println!("Private key: {:?}", key);
        }
    }
    Ok(())
}
