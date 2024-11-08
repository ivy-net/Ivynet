use ivynet_core::{
    avs::{
        config::{NodeConfig, NodeType},
        eigenda::{EigenDAConfig, EigenDANode},
        lagrange::Lagrange,
        mach_avs::AltLayer,
        names::AvsName,
        AvsProvider, AvsVariant, IvyNode,
    },
    config::{IvyConfig, Service, StartMode},
    eigen::contracts::delegation_manager::OperatorDetails,
    error::IvyError,
    ethers::{signers::Signer, types::Chain},
    grpc::{
        backend::backend_client::BackendClient,
        client::create_channel,
        ivynet_api::{
            ivy_daemon_avs::{
                avs_server::Avs, AttachRequest, AvsInfo, AvsInfoRequest, AvsInfoResponse,
                RegisterRequest, SelectAvsRequest, SetupRequest, StartRequest, StopRequest,
                UnregisterRequest,
            },
            ivy_daemon_operator::{
                operator_server::Operator, DelegatableSharesRequest, DelegatableSharesResponse,
                OperatorDetailsRequest, OperatorDetailsResponse, OperatorSharesRequest,
                OperatorSharesResponse, SetBlsKeyfilePathRequest, SetBlsKeyfilePathResponse,
                SetEcdsaKeyfilePathRequest, SetEcdsaKeyfilePathResponse, Shares,
            },
            ivy_daemon_types::RpcResponse,
        },
        tonic::{self, Request, Response, Status},
    },
    keychain::{KeyName, Keychain},
    messenger::BackendMessenger,
    node_manager::NodeManager,
    rpc_management::connect_provider,
    utils::try_parse_chain,
    wallet::IvyWallet,
};
use std::{borrow::BorrowMut, iter::zip, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, RwLock};

use crate::init::set_backend_connection;

#[derive(Debug, Clone)]
pub struct IvynetService {
    nodes: Arc<RwLock<NodeManager>>,
    config: Arc<Mutex<IvyConfig>>,
}

impl IvynetService {
    pub fn new(nodes: Arc<RwLock<NodeManager>>, config: &IvyConfig) -> Self {
        Self { nodes, config: Arc::new(Mutex::new(config.clone())) }
    }
}

// TODO: Granular setting chain and AVS, or is requiring both accepable?
#[tonic::async_trait]
impl Avs for IvynetService {
    async fn avs_info(
        &self,
        _request: Request<AvsInfoRequest>,
    ) -> Result<Response<AvsInfoResponse>, Status> {
        let provider = self.nodes.read().await;

        let response_vec: Vec<AvsInfo> = provider
            .iter()
            .map(|(k, v)| AvsInfo {
                running: v.is_running(),
                avs_type: v.node_type().to_string(),
                chain: v.chain().to_string(),
            })
            .collect();

        let response = AvsInfoResponse { avs_info: response_vec };
        Ok(Response::new(response))
    }

    async fn setup(
        &self,
        _request: Request<SetupRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        todo!();
    }

    async fn start(&self, request: Request<StartRequest>) -> Result<Response<RpcResponse>, Status> {
        let req = request.into_inner();
        let config_file = PathBuf::from(req.config);
        let node_config = NodeConfig::load(config_file).map_err(IvyError::from)?;

        let mut lock = self.config.lock().await;
        let ivy_config = lock.borrow_mut();

        // TODO: This probably needs to be handled somewhere else.
        if ivy_config.identity_wallet().is_err() {
            set_backend_connection(ivy_config).await?;
        }

        // Set up backend client messenger
        let backend_client = BackendClient::new(
            create_channel(ivynet_core::grpc::client::Source::Uri(ivy_config.get_server_url()?), {
                let ca = ivy_config.get_server_ca();
                if ca.is_empty() {
                    None
                } else {
                    Some(ca)
                }
            })
            .await
            .map_err(IvyError::from)?,
        );

        let messenger =
            Some(BackendMessenger::new(backend_client.clone(), ivy_config.identity_wallet()?));

        match node_config.node_type() {
            NodeType::EigenDA => {
                let eigenda_config: EigenDAConfig = node_config.try_into()?;
                let keyfile_pw = req.extra_data.get("ecdsa_keyfile_pw").ok_or_else(|| {
                    Status::invalid_argument("Missing keyfile password in extra_data")
                })?;
                let node_name = eigenda_config.name();

                let mut node = EigenDANode::from_config(eigenda_config, keyfile_pw).await?;
                node.start().await?;

                let mut node_manager = self.nodes.write().await;
                node_manager.insert(node_name, IvyNode { node: Box::new(node), messenger });
            }
            _ => return Err(Status::invalid_argument("Unknown node type")),
        }
        // let mut provider = self.avs_provider.write().await;
        // provider.start().await?;

        // let mut c = self.config.lock().await;
        // if let Some(ref mut s) = &mut c.configured_service {
        //     s.autostart = StartMode::Yes;
        //     _ = c.store();
        // }
        // // TODO: Start Flow + not setup fallback
        let response = RpcResponse { response_type: 0, msg: "Avs started.".to_string() };
        Ok(Response::new(response))
    }

    async fn attach(
        &self,
        _request: Request<AttachRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        // let mut provider = self.avs_provider.write().await;
        // provider.attach().await?;

        // let mut c = self.config.lock().await;
        // if let Some(ref mut s) = &mut c.configured_service {
        //     s.autostart = StartMode::Attach;
        //     _ = c.store();
        // }
        // let response =
        //     RpcResponse { response_type: 0, msg: "AVS attached successfully.".to_string() };
        // Ok(Response::new(response))
        todo!()
    }

    async fn stop(&self, _request: Request<StopRequest>) -> Result<Response<RpcResponse>, Status> {
        // let mut provider = self.avs_provider.write().await;
        // provider.stop().await?;

        // let mut c = self.config.lock().await;
        // if let Some(ref mut s) = &mut c.configured_service {
        //     s.autostart = StartMode::No;
        //     _ = c.store();
        // }
        // let response = RpcResponse { response_type: 0, msg: "Avs stopped.".to_string() };
        // Ok(Response::new(response))
        todo!()
    }

    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        // let provider = self.avs_provider.read().await;
        // let req = request.into_inner();
        // let operator_key_path = Keychain::default().get_path(KeyName::Ecdsa(req.operator_key_name));
        // provider.register(operator_key_path, &req.operator_key_pass).await?;

        // // TODO: Opt-in flow
        // let response = RpcResponse { response_type: 0, msg: "Register success.".to_string() };
        // Ok(Response::new(response))
        todo!()
    }

    async fn unregister(
        &self,
        _request: Request<UnregisterRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        // let provider = self.avs_provider.read().await;
        // // TODO: ask about storing 'config' in the provider
        // let config = IvyConfig::load_from_default_path().map_err(IvyError::from)?;
        // provider.unregister(&config).await?;

        // // TODO: Opt-out flow
        // let response = RpcResponse { response_type: 0, msg: "Unregister success.".to_string() };
        // Ok(Response::new(response))
        todo!()
    }

    // TODO: Running check stop
    // TODO: On bad netowork, don't change
    async fn select_avs(
        &self,
        request: Request<SelectAvsRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        // TODO: Clean this up if possible, complexity comes from needing to synchronize the rpc +
        // chain id between provider, signer, and AVS instance.
        // let req = request.into_inner();
        // let (avs, chain) = (req.avs, try_parse_chain(&req.chain)?);

        // {
        //     let mut provider = self.avs_provider.write().await;
        //     let signer = provider.provider.signer().clone();

        //     let avs_name = AvsName::try_from(avs.as_str());
        //     let avs_instance: Box<dyn AvsVariant> = match avs_name {
        //         Ok(AvsName::EigenDA) => Box::new(EigenDA::new_from_chain(chain)),
        //         Ok(AvsName::AltLayer) => Box::new(AltLayer::new_from_chain(chain)),
        //         Ok(AvsName::LagrangeZK) => Box::new(Lagrange::new_from_chain(chain)),
        //         _ => return Err(IvyError::InvalidAvsType(avs.to_string()).into()),
        //     };
        //     let new_ivy_provider = connect_provider(
        //         avs_instance.rpc_url().expect("Provider without RPC").as_ref(),
        //         Some(signer),
        //     )
        //     .await?;

        //     provider.set_avs(avs_instance, new_ivy_provider.into()).await?;

        //     let mut c = self.config.lock().await;
        //     c.configured_service =
        //         Some(Service { service: avs_name.unwrap(), chain, autostart: StartMode::No });
        //     _ = c.store();
        //     if let Some(ref mut s) = &mut c.configured_service {
        //         s.autostart = StartMode::No;
        //         _ = c.store();
        //     }
        // }

        // let response =
        //     RpcResponse { response_type: 0, msg: format!("AVS set: {} on chain {}", avs, chain) };
        // Ok(Response::new(response))
        todo!()
    }
}

#[tonic::async_trait]
impl Operator for IvynetService {
    async fn get_operator_details(
        &self,
        _request: Request<OperatorDetailsRequest>,
    ) -> Result<Response<OperatorDetailsResponse>, Status> {
        // let provider = self.avs_provider.read().await;
        // let operator_address = provider.provider.address();

        // // TODO: parallelize
        // let is_registered =
        //     provider.delegation_manager.is_operator(operator_address).await.map_err(|e| {
        //         Status::internal(format!("Failed to check if operator is registered: {}", e))
        //     })?;

        // let OperatorDetails {
        //     earnings_receiver,
        //     delegation_approver,
        //     staker_opt_out_window_blocks,
        // } = provider
        //     .delegation_manager
        //     .operator_details(operator_address)
        //     .await
        //     .map_err(|e| Status::internal(format!("Failed to get operator details: {}", e)))?;

        // let response = Response::new(OperatorDetailsResponse {
        //     operator: format!("{:?}", provider.provider.address()),
        //     is_registered,
        //     deprecated_earnings_receiver: format!("{:?}", earnings_receiver),
        //     delegation_approver: format!("{:?}", delegation_approver),
        //     staker_opt_out_window_blocks,
        // });
        // Ok(response)
        todo!()
    }

    async fn get_operator_shares(
        &self,
        _request: Request<OperatorSharesRequest>,
    ) -> Result<Response<OperatorSharesResponse>, Status> {
        // let provider = self.avs_provider.read().await;
        // let operator_address = provider.provider.address();
        // let manager = &provider.delegation_manager;
        // let strategies = manager.all_strategies().map_err(|e| {
        //     Status::internal(format!("Failed to get all strategies for operator shares: {}", e))
        // })?;
        // let shares = manager
        //     .get_operator_shares(operator_address, strategies.clone())
        //     .await
        //     .map_err(|e| Status::internal(format!("Failed to get operator shares: {}", e)))?;
        // let operator_shares: Vec<Shares> = zip(strategies.iter(), shares.iter())
        //     .map(|(s, sh)| Shares { strategy: format!("{:?}", s), shares: sh.to_string() })
        //     .collect();
        // let response = Response::new(OperatorSharesResponse { operator_shares });
        // Ok(response)
        todo!()
    }

    async fn get_delegatable_shares(
        &self,
        _request: Request<DelegatableSharesRequest>,
    ) -> Result<Response<DelegatableSharesResponse>, Status> {
        // let provider = self.avs_provider.read().await;
        // let operator_address = provider.provider.address();
        // let manager = &provider.delegation_manager;
        // let (strategies, shares) = manager
        //     .get_delegatable_shares(operator_address)
        //     .await
        //     .map_err(|e| Status::internal(format!("Failed to get delegatable shares: {}", e)))?;
        // let delegatable_shares: Vec<Shares> = zip(strategies.iter(), shares.iter())
        //     .map(|(s, sh)| Shares { strategy: format!("{:?}", s), shares: sh.to_string() })
        //     .collect();
        // Ok(Response::new(DelegatableSharesResponse { delegatable_shares }))
        todo!()
    }

    async fn set_ecdsa_keyfile_path(
        &self,
        request: Request<SetEcdsaKeyfilePathRequest>,
    ) -> Result<Response<SetEcdsaKeyfilePathResponse>, Status> {
        // let mut provider = self.avs_provider.write().await;
        // if let Some(avs) = &provider.avs {
        //     if avs.is_running() {
        //         return Err(Status::failed_precondition("AVS must be stopped to set keyfile path"));
        //     }
        // }

        // let req = request.into_inner();
        // let path = req.keyfile_path;
        // let pass = req.keyfile_password;

        // let signer = IvyWallet::from_keystore(path.clone().into(), &pass)?;

        // // Update provider
        // provider.with_signer(signer)?;
        // provider.with_keyfile_pw(Some(pass))?;

        // Ok(Response::new(SetEcdsaKeyfilePathResponse {}))
        todo!()
    }

    async fn set_bls_keyfile_path(
        &self,
        _request: Request<SetBlsKeyfilePathRequest>,
    ) -> Result<Response<SetBlsKeyfilePathResponse>, Status> {
        // TODO: This requres potential reworking of the way we pass the bls keyfile to the AVS.
        // Currently it's done through the .env file which is passed to the AVS, but we could also
        // potentially do it through a local ENV param or a config file. In either case, it should
        // be stored and passed somewhere outside of the AVS env file as this is a common param
        // needed by many AVS types.
        todo!();
    }
}
