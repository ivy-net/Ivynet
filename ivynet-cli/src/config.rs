use clap::Parser;
use dialoguer::{Input, Password};
use ethers::{types::Chain, utils::hex::ToHex as _};
use ivynet_core::{
    config::{self, IvyConfig},
    metadata::{self, Metadata},
    wallet::IvyWallet,
};

use crate::error::Error;

#[derive(Parser, Debug, Clone)]
pub enum ConfigCommands {
    #[command(name = "import-key", about = "Import and save as your default Ethereum private key with a password")]
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
    #[command(name = "get-default-public", about = "Get the current default saved keypair's Ethereum address")]
    GetDefaultEthAddress,
    #[command(name = "get-default-private", about = "Get the current default saved private key")]
    GetDefaultPrivateKey,
    #[command(
        name = "set-rpc",
        about = "Set default URLs to use when connecting to 'mainnet', 'holesky', and 'local' RPC urls"
    )]
    SetRpc {
        network: String,
        rpc_url: String,
    },
    #[command(name = "get-rpc", about = "Get the current default RPC URL for 'mainnet', 'holesky', or 'local'")]
    GetRpc {
        network: String,
    },
    #[command(
        name = "get-sys-info",
        about = "Get the number of CPU cores, memory, and free disk space on the current machine"
    )]
    #[command(name = "set-metadata", about = "Set metadata")]
    SetMetadata {
        metadata_uri: Option<String>,
        logo_uri: Option<String>,
        favicon_uri: Option<String>,
    },
    #[command(name = "get-metadata", about = "Get metadata")]
    GetMetadata,
    GetSysInfo,
}

pub fn parse_config_subcommands(subcmd: ConfigCommands, config: &mut IvyConfig, chain: Chain) -> Result<(), Error> {
    match subcmd {
        ConfigCommands::ImportPrivateKey { private_key, keyname, password } => {
            let wallet = IvyWallet::from_private_key(private_key)?;
            let (keyname, pass) = get_credentials(keyname, password);
            let (prv_key_path, pub_key_path) = wallet.encrypt_and_store(keyname, pass)?;
            config.default_private_keyfile = prv_key_path;
            config.default_public_keyfile = pub_key_path;
        }
        ConfigCommands::CreatePrivateKey { store, keyname, password } => {
            let wallet = IvyWallet::new();
            let priv_key = wallet.to_private_key();
            println!("Private key: {:?}", priv_key);
            let addr = wallet.address();
            println!("Public Address: {:?}", addr);
            if store {
                let (keyname, pass) = get_credentials(keyname, password);
                let (prv_key_path, pub_key_path) = wallet.encrypt_and_store(keyname, pass)?;
                config.default_private_keyfile = prv_key_path;
                config.default_public_keyfile = pub_key_path;
            }
        }
        ConfigCommands::SetRpc { network: _, rpc_url } => {
            config.set_rpc_url(chain, &rpc_url)?;
        }
        ConfigCommands::GetRpc { network } => {
            println!(
                "Url for {network} is {}",
                config.get_rpc_url(network.parse::<Chain>().expect("Wrong network name provided"))?
            );
        }
        ConfigCommands::GetDefaultEthAddress => {
            println!(
                "Public Key: {}",
                IvyWallet::address_from_file(config.default_public_keyfile.clone())?.encode_hex::<String>()
            );
        }
        ConfigCommands::GetDefaultPrivateKey => {
            let pass = Password::new().with_prompt("Enter a password to the private key").interact()?;
            let wallet = IvyWallet::from_keystore(config.default_private_keyfile.clone(), pass)?;
            println!("Private key: {:?}", wallet.to_private_key());
        }
        ConfigCommands::SetMetadata { metadata_uri, logo_uri, favicon_uri } => {
            let metadata_uri = if let Some(a) = metadata_uri { a } else { "".to_string() };
            let logo_uri = if let Some(a) = logo_uri { a } else { "".to_string() };
            let favicon_uri = if let Some(a) = favicon_uri { a } else { "".to_string() };
            config.metadata = Metadata::new(&metadata_uri, &logo_uri, &favicon_uri);
        }
        ConfigCommands::GetMetadata => {
            let metadata = &config.metadata;
            println!("{metadata:?}");
        }
        ConfigCommands::GetSysInfo => {
            let (cpus, mem_info, disk_info) = config::get_system_information()?;
            println!(" --- System Information: --- ");
            println!("CPU Cores: {cpus}");
            println!("Memory Information:");
            println!("  Total: {mem_info}");
            println!("Disk Information:");
            println!("  Free: {disk_info}");
            println!(" --------------------------- ");
        }
    };
    Ok(())
}

fn get_credentials(keyname: Option<String>, password: Option<String>) -> (String, String) {
    match (keyname, password) {
        (None, None) => (
            Input::new().with_prompt("Enter a name for the key").interact_text().expect("No keyname provided"),
            Password::new()
                .with_prompt("Enter a password to the private key")
                .interact()
                .expect("No password provided"),
        ),
        (None, Some(pass)) => {
            (Input::new().with_prompt("Enter a name for the key").interact_text().expect("No keyname provided"), pass)
        }
        (Some(keyname), None) => (
            keyname,
            Password::new()
                .with_prompt("Enter a password to the private key")
                .interact()
                .expect("No password provided"),
        ),
        (Some(keyname), Some(pass)) => (keyname, pass),
    }
}
