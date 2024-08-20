use clap::Parser;
use ivynet_core::{
    config::{self, IvyConfig},
    error::IvyError,
    ethers::types::Chain,
    grpc::{
        backend::backend_client::BackendClient,
        client::{create_channel, Request, Source, Uri},
        messages::RegistrationCredentials,
    },
    metadata::Metadata,
    utils::try_parse_chain,
};

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
    #[command(name = "register", about = "Register node on IvyNet server using IvyNet details")]
    Register {
        /// Email address registered at IvyNet portal
        #[arg(long, env = "IVYNET_EMAIL")]
        email: String,

        /// Password to IvyNet account
        #[arg(long, env = "IVYNET_PASSWORD")]
        password: String,
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
}

pub async fn parse_config_subcommands(
    subcmd: ConfigCommands,
    config: IvyConfig,
    server_url: Uri,
    server_ca: Option<&String>,
) -> Result<(), Error> {
    match subcmd {
        ConfigCommands::Set { command } => {
            let _ = parse_config_setter_commands(command, config);
        }
        ConfigCommands::Get { command } => {
            let _ = parse_config_getter_commands(command, &config);
        }
        ConfigCommands::Register { email, password } => {
            let config = IvyConfig::load_from_default_path().map_err(IvyError::from)?;
            let public_key = config.identity_wallet()?.address();
            let mut backend =
                BackendClient::new(create_channel(Source::Uri(server_url), server_ca).await?);
            backend
                .register(Request::new(RegistrationCredentials {
                    email,
                    password,
                    public_key: public_key.as_bytes().to_vec(),
                }))
                .await?;
            println!("Node registered");
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
            let chain = try_parse_chain(&chain)?;
            config.set_rpc_url(chain, &rpc_url)?;
            config.store().map_err(IvyError::from)?;
        }
        ConfigSetCommands::Metadata { metadata_uri, logo_uri, favicon_uri } => {
            let metadata_uri = metadata_uri.unwrap_or("".to_string());
            let logo_uri = logo_uri.unwrap_or("".to_string());
            let favicon_uri = favicon_uri.unwrap_or("".to_string());
            config.metadata = Metadata::new(&metadata_uri, &logo_uri, &favicon_uri);
            config.store().map_err(IvyError::from)?;
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
                config.get_rpc_url(chain.parse::<Chain>().expect("Wrong network name provided"))?
            );
        }
        ConfigGetCommands::Metadata {} => {
            let metadata = &config.metadata;
            println!("{metadata:?}");
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
            println!("{config:?}");
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
                "http://localhost:50051".parse().unwrap(),
                None,
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
                "http://localhost:50051".parse().unwrap(),
                None,
            )
            .await;

            assert!(result.is_ok());
        })
        .await;
    }
}
