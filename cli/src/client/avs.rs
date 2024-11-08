use std::collections::HashMap;

use ivynet_core::{
    avs::{
        config::{NodeConfig, NodeType},
        eigenda::EigenDAConfig,
    },
    error::IvyError,
    grpc::{
        ivynet_api::{
            ivy_daemon_avs::{
                avs_client::AvsClient as AvsClientRaw, AttachRequest, AvsInfoRequest,
                AvsInfoResponse, RegisterRequest, SelectAvsRequest, StartRequest, StopRequest,
                UnregisterRequest,
            },
            ivy_daemon_types::RpcResponse,
        },
        tonic::{transport::Channel, Request, Response},
    },
};

pub struct AvsClient(AvsClientRaw<Channel>);

impl AvsClient {
    pub fn new(channel: Channel) -> Self {
        Self(AvsClientRaw::new(channel))
    }

    pub async fn avs_info(&mut self) -> Result<Response<AvsInfoResponse>, IvyError> {
        let request = Request::new(AvsInfoRequest {});
        let response = self.0.avs_info(request).await?;
        Ok(response)
    }

    pub async fn register(
        &mut self,
        operator_key_name: String,
        operator_key_pass: String,
    ) -> Result<Response<RpcResponse>, IvyError> {
        let request = Request::new(RegisterRequest { operator_key_name, operator_key_pass });
        let response = self.0.register(request).await?;
        Ok(response)
    }

    pub async fn unregister(&mut self) -> Result<Response<RpcResponse>, IvyError> {
        let request = Request::new(UnregisterRequest {});
        let response = self.0.unregister(request).await?;
        Ok(response)
    }

    pub async fn start(
        &mut self,
        avs: Option<String>,
        chain: Option<String>,
    ) -> Result<Response<RpcResponse>, IvyError> {
        if let (Some(avs), Some(chain)) = (avs.clone(), chain.clone()) {
            let request = Request::new(SelectAvsRequest { avs, chain });
            let _ = self.0.select_avs(request).await?;
        }

        let config_files = NodeConfig::all().map_err(IvyError::from)?;

        let config_names: Vec<String> = config_files.iter().map(|c| c.name()).collect();

        let selected = dialoguer::Select::new()
            .with_prompt("Select a node configuration")
            .items(&config_names)
            .default(0)
            .interact()
            .map_err(IvyError::from)?;

        // This can probably be made more efficient, we load the config here once for some
        // logic handling and then re-load it on the daemon side. Consider passing the whole
        // config.

        let selected_config_path = config_files[selected].path();

        let config = NodeConfig::load(selected_config_path.clone()).map_err(IvyError::from)?;

        let extra_data: HashMap<String, String> = match config.node_type() {
            NodeType::EigenDA => {
                let config = EigenDAConfig::try_from(config).map_err(IvyError::from)?;
                let mut extra_data = HashMap::new();
                let ecdsa_keyfile_pw = dialoguer::Password::new()
                    .with_prompt(format!(
                        "Enter the password for keyfile {:?}",
                        config
                            .ecdsa_keyfile
                            .file_stem()
                            .expect("Could not extract filename from path.")
                    ))
                    .interact()
                    .map_err(IvyError::from)?;
                extra_data.insert("ecdsa_keyfile_pw".to_string(), ecdsa_keyfile_pw);
                extra_data
            }
            _ => HashMap::new(),
        };

        let request = Request::new(StartRequest {
            config: selected_config_path.display().to_string(),
            extra_data,
        });
        let response = self.0.start(request).await?;
        Ok(response)
    }

    pub async fn stop(&mut self) -> Result<Response<RpcResponse>, IvyError> {
        let request = Request::new(StopRequest {});
        let response = self.0.stop(request).await?;
        Ok(response)
    }

    pub async fn select_avs(
        &mut self,
        avs: String,
        chain: String,
    ) -> Result<Response<RpcResponse>, IvyError> {
        let request = Request::new(SelectAvsRequest { avs, chain });
        let response = self.0.select_avs(request).await?;
        Ok(response)
    }

    pub async fn attach(
        &mut self,
        avs: Option<String>,
        chain: Option<String>,
    ) -> Result<Response<RpcResponse>, IvyError> {
        if let (Some(avs), Some(chain)) = (avs.clone(), chain.clone()) {
            let request = Request::new(SelectAvsRequest { avs, chain });
            let _ = self.0.select_avs(request).await?;
        }

        let request = Request::new(AttachRequest { avs, chain });
        let response = self.0.attach(request).await?;

        Ok(response)
    }
}
