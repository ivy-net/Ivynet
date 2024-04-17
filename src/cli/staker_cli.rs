use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub(crate) enum StakerCommands {
    #[command(
        name = "get-staker-shares",
        about = "Get data on a staker's strategy choices and their stake in each one"
    )]
    GetStakerShares {
        private_key: String,
    },
    #[command(
        name = "get-my-shares",
        about = "Get data on the saved keypair's current strategy and stake"
    )]
    GetMyShares,
}

pub async fn parse_staker_subcommands(subcmd: StakerCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcmd {
        StakerCommands::GetStakerShares { private_key } => todo!(),
        StakerCommands::GetMyShares => todo!(),
    }
    Ok(())
}