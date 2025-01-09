use anyhow::{Error as AnyError, Result};
use ivynet_core::{
    avs::{
        commands::NodeCommands, config::NodeConfig, eigenda::EigenDAConfig,
        lagrange::config::LagrangeConfig,
    },
    error::IvyError,
};
use ivynet_node_type::NodeType;

pub async fn parse_avs_subcommands(subcmd: NodeCommands) -> Result<(), AnyError> {
    match subcmd {
        NodeCommands::Configure { node_type } => match node_type {
            NodeType::EigenDA => {
                let config = EigenDAConfig::new_from_prompt().await?;
                println!("Setup complete, EigenDA config saved to {}", config.path.display());
                NodeConfig::EigenDA(config).store();
            }
            NodeType::LagrangeZkWorker => {
                let config = LagrangeConfig::new_from_prompt().await?;
                println!(
                    "Setup complete, Lagrange holesky config saved to {}",
                    config.path.display()
                );
                NodeConfig::LagrangeZkWorkerHolesky(config).store();
            }
            _ => unimplemented!("Node type not implemented: {:?}", node_type),
        },

        NodeCommands::Info {} => {
            todo!()
        }
        NodeCommands::Start {} => {
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
        NodeCommands::Stop {} => {
            todo!()
        }
        _ => unimplemented!("Command not implemented: {:?}", subcmd),
    }
    Ok(())
}
