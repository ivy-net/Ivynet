use clap::Parser;
use ethers::types::Chain;
use ivynet_grpc::client::Uri;
use ivynet_io::{read_toml, write_toml, IoError};
use ivynet_signer::{IvyWallet, IvyWalletError};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error as ThisError;
use uuid::Uuid;

pub static DEFAULT_CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let path = dirs::home_dir().expect("Could not get a home directory");
    path.join(".ivynet")
});

use crate::{error::Error, ivy_machine::SystemInformation, metadata::Metadata};

#[derive(Parser, Debug, Clone)]
pub enum ConfigCommands {
    #[command(name = "set", about = "Set configuration values for either RPC or metadata")]
    Set {
        #[command(subcommand)]
        command: ConfigSetCommands,
    },
    #[command(
        name = "get",
        about = "Get configuration values for RPC, metadata, config or get system info"
    )]
    Get {
        #[command(subcommand)]
        command: ConfigGetCommands,
    },
}

#[derive(Parser, Debug, Clone)]
pub enum ConfigSetCommands {
    #[command(
        name = "rpc",
        about = "Set default URLs to use when connecting to 'mainnet', 'holesky', and 'local' RPC urls <CHAIN> <RPC_URL>"
    )]
    Rpc { chain: String, rpc_url: String },
    #[command(name = "metadata", about = "Set metadata for EigenLayer Operator")]
    Metadata { metadata_uri: Option<String>, logo_uri: Option<String>, favicon_uri: Option<String> },
    #[command(name = "server_url", about = "Set backend server connection url")]
    ServerUrl { server_url: Uri },
    #[command(name = "server_ca", about = "Set backend server certificate")]
    ServerCa { server_ca: String },
    #[command(name = "identity_key", about = "Set backend connection identity key")]
    IdentityKey { identity_key: String },
}

#[derive(Parser, Debug, Clone)]
pub enum ConfigGetCommands {
    #[command(
        name = "rpc",
        about = "Get default URLs to use when connecting to 'mainnet', 'holesky', and 'local' RPC urls <CHAIN>"
    )]
    Rpc { chain: String },
    #[command(name = "metadata", about = "Get local metadata")]
    Metadata,
    #[command(name = "config", about = "Get all config data")]
    Config,
    #[command(name = "sys-info", about = "Get system information")]
    SysInfo,
    #[command(name = "backend", about = "Get backend connection information")]
    Backend,
}

pub async fn parse_config_subcommands(
    subcmd: ConfigCommands,
    config: IvyConfig,
) -> Result<(), Error> {
    match subcmd {
        ConfigCommands::Set { command } => {
            let _ = parse_config_setter_commands(command, config);
        }
        ConfigCommands::Get { command } => {
            let _ = parse_config_getter_commands(command, &config);
        }
    };
    Ok(())
}

fn parse_config_setter_commands(
    subsetter: ConfigSetCommands,
    mut config: IvyConfig,
) -> Result<(), Error> {
    match subsetter {
        ConfigSetCommands::Rpc { chain, rpc_url } => {
            let chain =
                chain.parse::<Chain>().map_err(|e| Error::ChainParseError(e.to_string()))?;
            config.set_default_rpc_url(chain, &rpc_url)?;
            config.store()?;
        }
        ConfigSetCommands::Metadata { metadata_uri, logo_uri, favicon_uri } => {
            let metadata_uri = metadata_uri.unwrap_or("".to_string());
            let logo_uri = logo_uri.unwrap_or("".to_string());
            let favicon_uri = favicon_uri.unwrap_or("".to_string());
            config.metadata = Metadata::new(&metadata_uri, &logo_uri, &favicon_uri);
            config.store()?;
        }
        ConfigSetCommands::ServerUrl { server_url } => {
            config.backend_info.server_url = server_url.to_string();
            config.store()?;
        }
        ConfigSetCommands::ServerCa { server_ca } => {
            config.backend_info.server_ca = server_ca;
            config.store()?;
        }
        ConfigSetCommands::IdentityKey { identity_key } => {
            config.backend_info.identity_key = identity_key;
            config.store()?;
        }
    }
    Ok(())
}

fn parse_config_getter_commands(
    subgetter: ConfigGetCommands,
    config: &IvyConfig,
) -> Result<(), Error> {
    match subgetter {
        ConfigGetCommands::Rpc { chain } => {
            println!(
                "Url for {chain} is {}",
                config.get_default_rpc_url(
                    chain.parse::<Chain>().expect("Wrong network name provided")
                )?
            );
        }
        ConfigGetCommands::Metadata {} => {
            let metadata = &config.metadata;
            println!("{metadata:#?}");
        }
        ConfigGetCommands::SysInfo {} => {
            let sys_info = SystemInformation::from_system();
            println!(" --- System Information: --- ");
            println!("{sys_info:#?}");
            println!(" --------------------------- ");
        }
        ConfigGetCommands::Config {} => {
            println!("{config:#?}");
        }
        ConfigGetCommands::Backend {} => {
            println!("{:#?}", config.backend_info);
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackendInfo {
    pub server_url: String,
    pub server_ca: String,
    /// Identification key that node uses for server communications
    pub identity_key: String,
}

// TODO: Change rpc urls to hashmap or remove entirely
// add reference to keyfile for identity keys instead of using provider id
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IvyConfig {
    /// Storage path for serialized config file
    path: PathBuf,
    /// Machines Id
    pub machine_id: Uuid,
    /// RPC URL for mainnet
    pub mainnet_rpc_url: String,
    /// RPC URL for holesky
    pub holesky_rpc_url: String,
    // RPC URL for local development
    pub local_rpc_url: String,
    /// Metadata for the operator
    pub metadata: Metadata,
    /// Web server information
    pub backend_info: BackendInfo,
}

impl Default for IvyConfig {
    fn default() -> Self {
        Self {
            path: DEFAULT_CONFIG_PATH.to_owned(),
            machine_id: Uuid::new_v4(),
            mainnet_rpc_url: "https://rpc.flashbots.net/fast".to_string(),
            holesky_rpc_url: "https://eth-holesky.public.blastapi.io".to_string(),
            local_rpc_url: "http://localhost:8545".to_string(),
            metadata: Metadata::default(),
            backend_info: BackendInfo {
                server_url: "https://api1.test.ivynet.dev".into(),
                server_ca: "".into(),
                identity_key: "".into(),
            },
        }
    }
}

impl IvyConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_at_path(path: PathBuf) -> Self {
        Self { path, ..Default::default() }
    }

    pub fn load(path: PathBuf) -> Result<Self, ConfigError> {
        let config: Self = read_toml(&path)?;
        Ok(config)
    }

    pub fn load_from_default_path() -> Result<Self, ConfigError> {
        let config_path = DEFAULT_CONFIG_PATH.to_owned().join("ivy-config.toml");
        //Previous impl built a bad path - let this error properly
        Self::load(config_path)
    }

    pub fn store(&self) -> Result<(), ConfigError> {
        // TODO: Assert identity key is None on save
        let config_path = self.path.clone().join("ivy-config.toml");
        write_toml(&config_path, self)?;
        Ok(())
    }

    pub fn set_default_rpc_url(&mut self, chain: Chain, rpc: &str) -> Result<(), Error> {
        match chain {
            Chain::Mainnet => {
                println!("Setting mainnet rpc url to: {}", rpc);
                self.mainnet_rpc_url = rpc.to_string();
            }
            Chain::Holesky => {
                println!("Setting holesky rpc url to: {}", rpc);
                self.holesky_rpc_url = rpc.to_string();
            }
            Chain::AnvilHardhat => {
                println!("Setting local rpc url to: {}", rpc);
                self.local_rpc_url = rpc.to_string();
            }
            _ => return Err(Error::ChainUnimplemented(chain.to_string())),
        }
        Ok(())
    }

    pub fn get_default_rpc_url(&self, chain: Chain) -> Result<String, Error> {
        match chain {
            Chain::Mainnet => Ok(self.mainnet_rpc_url.clone()),
            Chain::Holesky => Ok(self.holesky_rpc_url.clone()),
            Chain::AnvilHardhat => Ok(self.local_rpc_url.clone()),
            _ => Err(Error::ChainUnimplemented(chain.to_string())),
        }
    }

    pub fn get_path(&self) -> PathBuf {
        self.path.clone()
    }

    /// Get the path to the directory containing the ivy-config.toml file.
    pub fn get_dir(&self) -> PathBuf {
        self.path.clone()
    }

    /// Get the path to the ivy-config.toml file.
    pub fn get_file(&self) -> PathBuf {
        self.path.join("ivy-config.toml")
    }

    pub fn identity_wallet(&self) -> Result<IvyWallet, Error> {
        Ok(IvyWallet::from_private_key(self.backend_info.identity_key.clone())?)
    }

    pub fn set_server_url(&mut self, url: String) {
        self.backend_info.server_url = url;
    }

    pub fn get_server_url(&self) -> Result<Uri, Error> {
        Uri::try_from(self.backend_info.server_url.clone()).map_err(|_| Error::InvalidUri)
    }

    pub fn set_server_ca(&mut self, ca: String) {
        self.backend_info.server_ca = ca;
    }

    pub fn get_server_ca(&self) -> String {
        self.backend_info.server_ca.clone()
    }

    pub fn uds_dir(&self) -> String {
        format!("{}/ivynet.ipc", self.path.display())
    }
}

#[derive(ThisError, Debug)]
pub enum ConfigError {
    #[error(transparent)]
    ConfigIo(#[from] IoError),
    #[error(transparent)]
    WalletFetchError(#[from] IvyWalletError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{future::Future, path::PathBuf};
    use tokio::fs;

    pub async fn build_test_dir<F, Fut, T>(test_dir: &str, test_logic: F) -> T
    where
        F: FnOnce(PathBuf) -> Fut,
        Fut: Future<Output = T>,
    {
        let test_path = std::env::current_dir().unwrap().join(format!("testing{}", test_dir));
        fs::create_dir_all(&test_path).await.expect("Failed to create testing_temp directory");
        let result = test_logic(test_path.clone()).await;
        fs::remove_dir_all(test_path).await.expect("Failed to delete testing_temp directory");

        result
    }

    #[test]
    fn test_load_config_error() {
        let path = PathBuf::from("nonexistent");
        let config = IvyConfig::load(path);
        println!("{:?}", config);
        assert!(config.is_err());
    }

    #[test]
    fn test_uds_dir() {
        let config = super::IvyConfig::default();
        let path_str = config.path.display().to_string();
        let uds_dir = config.uds_dir();
        assert_eq!(uds_dir, path_str + "/ivynet.ipc");
        println!("{}", uds_dir);
    }

    #[tokio::test]
    async fn test_rpc_functionality() {
        let test_dir = "test_rpc_functionality";
        build_test_dir(test_dir, |test_path| async move {
            let config = IvyConfig::new_at_path(test_path.clone());

            let result = parse_config_subcommands(
                ConfigCommands::Set {
                    command: ConfigSetCommands::Rpc {
                        chain: "mainnet".to_string(),
                        rpc_url: "http://localhost:8545".to_string(),
                    },
                },
                config,
            )
            .await;

            assert!(result.is_ok());

            let config =
                IvyConfig::load(test_path.join("ivy-config.toml")).expect("Failed to load config");

            let result = parse_config_subcommands(
                ConfigCommands::Get {
                    command: ConfigGetCommands::Rpc { chain: "mainnet".to_string() },
                },
                config,
            )
            .await;

            assert!(result.is_ok());
        })
        .await;
    }
}
