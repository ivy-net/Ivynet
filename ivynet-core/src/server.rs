// Dummy implementation of a server for managing an AVS instance.
// For now, each of these will initialize its own AVS instance, run, and then shutdown. Future
// iteratons will work on along-running AVS instance.

pub struct AvsData<'a> {
    id: &'a str,
    config: &'a IvyConfig,
    chain: ethers::types::Chain
}

pub enum AvsCommands {
    Setup {avs: &str},
    Start { avs: &str },
    Stop { avs: &str },
    Optin {avs: &str },
    Optout { avs: &str },
}

pub async fn handle_avs_command(op: AvsCommands, config: &IvyConfig, chain: Chain avs: &str) -> {
    match op {
        AvsCommands::Setup => todo!(),
        AvsCommands::Start => todo!(),
        AvsCommands::Stop => todo!(),
        AvsCommands::Optin => {

        },
        AvsCommands::Optout => todo!(),
    }
}
