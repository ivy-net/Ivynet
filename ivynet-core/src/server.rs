// Dummy implementation of a server for managing an AVS instance.
// For now, each of these will initialize its own AVS instance, run, and then shutdown. Future
// iteratons will work on along-running AVS instance. Currently some redundant code due to this in
// cli/avs.rs, to be cleaned.

use std::{str::FromStr, sync::Arc};

use ethers::{providers::Middleware, types::Chain};

use crate::{
    avs::{eigenda::EigenDA, instance::AvsInstance, mach_avs::AltLayer, AvsProvider},
    config::IvyConfig,
    error::IvyError,
    rpc_management::connect_provider,
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
    op: AvsHandleCommands,
    id: &str,
    config: &IvyConfig,
    chain: Chain,
    wallet: Option<IvyWallet>,
) -> Result<(), IvyError> {
    let mut avs_dir = dirs::home_dir().expect("Could not get a home directory");
    avs_dir.push(".eigenlayer");

    let provider = connect_provider(&config.get_rpc_url(chain)?, wallet).await?;
    let chain: Chain = Chain::try_from(provider.get_chainid().await?).unwrap_or_default();

    let avs = match Avs::from_str(id) {
        Ok(Avs::EigenDA) => AvsInstance::EigenDA(EigenDA::default()),
        Ok(Avs::AltLayer) => AvsInstance::AltLayer(AltLayer::default()),
        Err(_) => return Err(IvyError::AvsParseError),
    };

    let avs_provider = AvsProvider::new(chain, avs, Arc::new(provider));

    match op {
        AvsHandleCommands::Setup => avs_provider.setup(config).await?,
        AvsHandleCommands::Start => avs_provider.start(config).await?,
        AvsHandleCommands::Stop => avs_provider.stop(config).await?,
        AvsHandleCommands::Optin => avs_provider.opt_in(config).await?,
        AvsHandleCommands::Optout => avs_provider.opt_out(config).await?,
    }
    Ok(())
}
