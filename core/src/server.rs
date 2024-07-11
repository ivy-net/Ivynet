// Dummy implementation of a server for managing an AVS instance.
// For now, each of these will initialize its own AVS instance, run, and then shutdown. Future
// iteratons will work on along-running AVS instance. Currently some redundant code due to this in
// cli/avs.rs, to be cleaned.

use std::{str::FromStr, sync::Arc};

use crate::{
    avs::{commands::AvsCommands, eigenda::EigenDA, instance::AvsInstance, mach_avs::AltLayer, AvsProvider},
    config::IvyConfig,
    error::IvyError,
    rpc_management::connect_provider,
    utils::parse_chain,
    wallet::IvyWallet,
};

pub enum AvsHandleCommands {
    Setup,
    Start,
    Stop,
    Optin,
    Optout,
}

enum Avs {
    EigenDA,
    AltLayer,
}

impl FromStr for Avs {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "eigenda" => Ok(Avs::EigenDA),
            "altlayer" => Ok(Avs::AltLayer),
            _ => Err(()),
        }
    }
}

pub async fn handle_avs_command(
    op: AvsCommands,
    config: &IvyConfig,
    wallet: Option<IvyWallet>,
) -> Result<(), IvyError> {
    let mut avs_dir = dirs::home_dir().expect("Could not get a home directory");
    avs_dir.push(".eigenlayer");
    match op {
        AvsCommands::Setup { avs, chain } => {
            let avs = build_avs_provider(&avs, &chain, config, wallet).await?;
            avs.setup(config).await?;
        }
        AvsCommands::Optin { avs, chain } => {
            let avs = build_avs_provider(&avs, &chain, config, wallet).await?;
            avs.opt_in(config).await?;
        }
        AvsCommands::Optout { avs, chain } => {
            let avs = build_avs_provider(&avs, &chain, config, wallet).await?;
            avs.opt_out(config).await?;
        }
        AvsCommands::Start { avs, chain } => {
            let avs = build_avs_provider(&avs, &chain, config, wallet).await?;
            avs.start(config).await?;
        }
        AvsCommands::Stop { avs, chain } => {
            let avs = build_avs_provider(&avs, &chain, config, wallet).await?;
            avs.stop(config).await?;
        }
        AvsCommands::CheckStakePercentage { .. } => todo!(),
    }
    Ok(())
}

// TODO: This can probably be improved or put in a method
async fn build_avs_provider(
    id: &str,
    chain: &str,
    config: &IvyConfig,
    wallet: Option<IvyWallet>,
) -> Result<AvsProvider<AvsInstance>, IvyError> {
    let chain = parse_chain(chain);
    let provider = connect_provider(&config.get_rpc_url(chain)?, wallet).await?;
    let avs_instance = match Avs::from_str(id) {
        Ok(Avs::EigenDA) => AvsInstance::EigenDA(EigenDA::default()),
        Ok(Avs::AltLayer) => AvsInstance::AltLayer(AltLayer::default()),
        Err(_) => return Err(IvyError::AvsParseError),
    };
    Ok(AvsProvider::new(chain, avs_instance, Arc::new(provider)))
}
