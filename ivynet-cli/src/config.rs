use clap::Parser;
use dialoguer::{Input, Password};
use ivynet_core::{
    config::{self, IvyConfig},
    error::IvyError,
    ethers::{types::Chain, utils::hex::ToHex as _},
    metadata::Metadata,
    wallet::IvyWallet,
};

use crate::{error::Error, utils::parse_chain};

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
        chain: String,
        rpc_url: String,
    },
    #[command(name = "get-rpc", about = "Get the current default RPC URL for 'mainnet', 'holesky', or 'local'")]
    GetRpc {
        chain: String,
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
    #[command(name = "get-config", about = "Get all config data")]
    GetConfig,
    GetSysInfo,
}

pub fn parse_config_subcommands(subcmd: ConfigCommands, config: &mut IvyConfig) -> Result<(), Error> {
    match subcmd {
        ConfigCommands::ImportPrivateKey { private_key, keyname, password } => {
            let wallet = IvyWallet::from_private_key(private_key)?;
            let (keyname, pass) = get_credentials(keyname, password);
            let (pub_key_path, prv_key_path) = wallet.encrypt_and_store(&config.get_path(), keyname, pass)?;
            config.default_private_keyfile = prv_key_path;
            config.default_public_keyfile = pub_key_path;
            config.store()?;
        }
        ConfigCommands::CreatePrivateKey { store, keyname, password } => {
            let wallet = IvyWallet::new();
            let priv_key = wallet.to_private_key();
            println!("Private key: {:?}", priv_key);
            let addr = wallet.address();
            println!("Public Address: {:?}", addr);
            if store {
                let (keyname, pass) = get_credentials(keyname, password);
                let (pub_key_path, prv_key_path) = wallet.encrypt_and_store(&config.get_path(), keyname, pass)?;
                config.default_private_keyfile = prv_key_path;
                config.default_public_keyfile = pub_key_path;
                config.store()?;
            }
        }
        ConfigCommands::SetRpc { chain, rpc_url } => {
            let chain = parse_chain(&chain);
            config.set_rpc_url(chain, &rpc_url)?;
            config.store()?;
        }
        ConfigCommands::GetRpc { chain } => {
            println!(
                "Url for {chain} is {}",
                config.get_rpc_url(chain.parse::<Chain>().expect("Wrong network name provided"))?
            );
        }
        ConfigCommands::GetDefaultEthAddress => {
            println!("Public Key: {:?}", IvyWallet::address_from_file(config.default_public_keyfile.clone())?);
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
        ConfigCommands::GetConfig => {
            println!("{config:?}")
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
