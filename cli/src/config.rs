use clap::Parser;
use ethers::types::Chain;
use ivynet_core::{
    config::{self, IvyConfig},
    metadata::Metadata,
};
use ivynet_grpc::client::Uri;

use crate::error::Error;

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
            let (cpus, mem_info, disk_info) = config::get_system_information()?;
            println!(" --- System Information: --- ");
            println!("CPU Cores: {cpus}");
            println!("Memory Information:");
            println!("  Total: {mem_info}");
            println!("Disk Information:");
            println!("  Free: {disk_info}");
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

    //Usage example within an async test

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
