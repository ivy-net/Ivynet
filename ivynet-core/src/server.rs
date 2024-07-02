// Dummy implementation of a server for managing an AVS instance.
// For now, each of these will initialize its own AVS instance, run, and then shutdown. Future
// iteratons will work on along-running AVS instance. Currently some redundant code due to this in
// cli/avs.rs, to be cleaned.

use std::sync::Arc;
use tonic::Request;

use crate::{
    avs::{commands::AvsCommands, instance::AvsType, AvsProvider},
    config::IvyConfig,
    error::IvyError,
    grpc::{
        client::create_channel,
        ivynet_api::ivy_daemon_avs::{
            avs_client::AvsClient, AvsInfoRequest, OptinRequest, OptoutRequest, SetAvsRequest, StartRequest,
            StopRequest,
        },
    },
    rpc_management::connect_provider,
    utils::parse_chain,
    wallet::IvyWallet,
};

pub async fn handle_avs_command(
    op: AvsCommands,
    config: &IvyConfig,
    wallet: Option<IvyWallet>,
) -> Result<(), IvyError> {
    let uri = tonic::transport::Uri::from_static("http://localhost:55501");
    match op {
        //TODO: Consider re-scoping the setup command for cleaner separation of concerns and less
        //boilerplate
        AvsCommands::Setup { avs, chain } => {
            let avs = build_avs_provider(Some(&avs), &chain, config, wallet).await?;
            avs.setup(config).await?;
        }
        AvsCommands::Info {} => {
            let mut client = AvsClient::new(create_channel(&uri, None));
            let request = Request::new(AvsInfoRequest {});
            let response = client.avs_info(request).await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::Optin { avs, chain } => {
            let mut client = AvsClient::new(create_channel(&uri, None));
            let request = Request::new(OptinRequest {});
            let response = client.opt_in(request).await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::Optout { avs, chain } => {
            let mut client = AvsClient::new(create_channel(&uri, None));
            let request = Request::new(OptoutRequest {});
            let response = client.opt_out(request).await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::Start { avs, chain } => {
            // TODO: Pass commands if present, mke cli args optional
            // Error flow
            let mut client = AvsClient::new(create_channel(&uri, None));
            let request = Request::new(StartRequest { avs: None, chain: None });
            let response = client.start(request).await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::Stop { avs, chain } => {
            let mut client = AvsClient::new(create_channel(&uri, None));
            let reqwest = Request::new(StopRequest {});
            let response = client.stop(reqwest).await?;
            print!("{:?}", response.into_inner());
        }
        AvsCommands::SetAvs { avs, chain } => {
            let mut client = AvsClient::new(create_channel(&uri, None));
            let request = Request::new(SetAvsRequest { avs, chain });
            let response = client.set_avs(request).await?;
            println!("{:?}", response.into_inner());
        }
        AvsCommands::CheckStakePercentage { .. } => todo!(),
    };
    Ok(())
}

// TODO: Builder pattern
pub async fn build_avs_provider(
    id: Option<&str>,
    chain: &str,
    config: &IvyConfig,
    wallet: Option<IvyWallet>,
) -> Result<AvsProvider, IvyError> {
    let chain = parse_chain(chain);
    let provider = connect_provider(&config.get_rpc_url(chain)?, wallet).await?;
    let avs_instance = if let Some(avs_id) = id { Some(AvsType::new(avs_id, chain)?) } else { None };
    Ok(AvsProvider::new(avs_instance, Arc::new(provider)))
}
