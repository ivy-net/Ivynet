use anyhow::{Error as AnyError, Result};
use ivynet_core::{
    avs::{
        commands::AvsCommands,
        config::{NodeConfig, NodeType},
        eigenda::EigenDAConfig,
    },
    error::IvyError,
};

pub async fn parse_avs_subcommands(subcmd: AvsCommands) -> Result<(), AnyError> {
    if let AvsCommands::Configure { node_type } = subcmd {
        match node_type {
            NodeType::EigenDA => {
                let config = EigenDAConfig::new_from_prompt().await?;
                NodeConfig::EigenDA(config).store();
            }
            _ => unimplemented!("Node type not implemented: {:?}", node_type),
        }
        return Ok(());
    }

    match subcmd {
        AvsCommands::Info {} => {
            todo!()
        }
        // TODO: Fix timeout issue
        AvsCommands::Register {} => {
            todo!()
        }
        AvsCommands::Unregister {} => {
            todo!()
        }
        AvsCommands::Start {} => {
            // Prompt user for config to start
            let config_files = NodeConfig::all().map_err(IvyError::from)?;

            let config_names: Vec<String> = config_files.iter().map(|c| c.name()).collect();

            let selected = dialoguer::Select::new()
                .with_prompt("Select a node configuration")
                .items(&config_names)
                .default(0)
                .interact()
                .map_err(IvyError::from)?;

            let selected_config_path = config_files[selected].path();

            let node_config = NodeConfig::load(&selected_config_path).map_err(IvyError::from)?;

            match node_config {
                NodeConfig::EigenDA(config) => config.start().await?,
                _ => unimplemented!("Node type not implemented: {:?}", node_config.node_type()),
            };
        }
        AvsCommands::Stop {} => {
            todo!()
        }
        _ => unimplemented!("Command not implemented: {:?}", subcmd),
    }
    Ok(())
}
