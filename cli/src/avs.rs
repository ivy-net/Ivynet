use anyhow::{Error as AnyError, Result};
use ivynet_core::avs::config::NodeConfig;

use crate::commands::NodeCommands;

pub async fn parse_avs_subcommands(subcmd: NodeCommands) -> Result<(), AnyError> {
    match subcmd {
        NodeCommands::Info {} => {
            todo!()
        }
        NodeCommands::Start {} => {
            // Prompt user for config to start
            let config_files = NodeConfig::all()?;
            let config_names: Vec<String> = config_files.iter().map(|c| c.name()).collect();

            let selected = dialoguer::Select::new()
                .with_prompt("Select a node configuration")
                .items(&config_names)
                .default(0)
                .interact()?;

            let selected_config_path = config_files[selected].path();

            let node_config = NodeConfig::load(&selected_config_path)?;

            match node_config {
                NodeConfig::EigenDA(config) => config.start().await?,
                _ => unimplemented!("Node type not implemented: {:?}", node_config.node_type()),
            };
        }
        NodeCommands::Stop {} => {
            todo!()
        }
        _ => unimplemented!("Command not implemented: {:?}", subcmd),
    }
    Ok(())
}
