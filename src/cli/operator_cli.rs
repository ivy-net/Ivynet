use clap::Parser;

use crate::rpc;

#[derive(Parser, Debug, Clone)]
pub(crate) enum OperatorCommands {
    #[command(name = "get_operator_details", about = "Get operator details")]
    GetOperatorDetails { address: String },
    #[command(
        name = "get_operator_stake",
        about = "Get an operator's total delineated stake per strategy"
    )]
    GetOperatorStake { address: String },
}

pub async fn parse_operator_subcommands(subcmd: OperatorCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        OperatorCommands::GetOperatorDetails { address } => {
            rpc::delegation_manager::get_operator_details(address).await?
        }
        OperatorCommands::GetOperatorStake { address } => todo!(),
    }
    Ok(())
}
