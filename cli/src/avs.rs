use crate::commands::NodeCommands;
use anyhow::{Error as AnyError, Result};

pub async fn parse_avs_subcommands(subcmd: NodeCommands) -> Result<(), AnyError> {
    match subcmd {
        NodeCommands::Info {} => {
            todo!()
        }
        NodeCommands::Start {} => {
            todo!()
        }
        NodeCommands::Stop {} => {
            todo!()
        }
        _ => unimplemented!("Command not implemented: {:?}", subcmd),
    }
}
