use ivynet_core::{
    avs::{eigenda::EigenDA, mach_avs::AltLayer, AvsProvider, AvsVariant},
    config::IvyConfig,
    eigen::contracts::delegation_manager::OperatorDetails,
    error::IvyError,
    ethers::{signers::Signer, types::Chain},
    grpc::{
        ivynet_api::{
            ivy_daemon_avs::{
                avs_server::Avs, AvsInfoRequest, AvsInfoResponse, RegisterRequest,
                SelectAvsRequest, SetupRequest, StartRequest, StopRequest, UnregisterRequest,
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
    rpc_management::connect_provider,
    utils::try_parse_chain,
    wallet::IvyWallet,
};
use std::{iter::zip, sync::Arc};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct IvynetService {
    avs_provider: Arc<RwLock<AvsProvider>>,
}

impl IvynetService {
    pub fn new(avs_provider: Arc<RwLock<AvsProvider>>) -> Self {
        Self { avs_provider }
    }
}

// TODO: Granular setting chain and AVS, or is requiring both accepable?
#[tonic::async_trait]
impl Avs for IvynetService {
    async fn avs_info(
        &self,
        _request: Request<AvsInfoRequest>,
    ) -> Result<Response<AvsInfoResponse>, Status> {
        let provider = self.avs_provider.read().await;
        let avs = &provider.avs;
        let (running, avs_type, chain) = if let Some(avs) = avs {
            let is_running = avs.running();
            let avs_type = avs.name();
            let chain = Chain::try_from(provider.provider.signer().chain_id())
                .expect("Unexpected chain ID parse failure");
            (is_running, avs_type, chain.to_string())
        } else {
            let avs_type = "None";
            let chain = "None";
            (false, avs_type, chain.to_string())
        };
        let response = AvsInfoResponse { running, avs_type: avs_type.to_string(), chain };
        Ok(Response::new(response))
    }

    async fn setup(
        &self,
        _request: Request<SetupRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        todo!();
    }

    async fn start(
        &self,
        _request: Request<StartRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        let mut provider = self.avs_provider.write().await;
        provider.start().await?;

        // TODO: Start Flow + not setup fallback
        let response = RpcResponse { response_type: 0, msg: "Avs started.".to_string() };
        Ok(Response::new(response))
    }

    async fn stop(&self, _request: Request<StopRequest>) -> Result<Response<RpcResponse>, Status> {
        let mut provider = self.avs_provider.write().await;
        let chain = provider.chain().await?;
        provider.stop(chain).await?;

        // TODO: Stop flow
        let response = RpcResponse { response_type: 0, msg: "Avs stopped.".to_string() };
        Ok(Response::new(response))
    }

    async fn register(
        &self,
        _request: Request<RegisterRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        let provider = self.avs_provider.read().await;
        // TODO: ask about storing 'config' in the provider
        let config = IvyConfig::load_from_default_path().map_err(IvyError::from)?;
        provider.register(&config).await?;

        // TODO: Opt-in flow
        let response = RpcResponse { response_type: 0, msg: "Register success.".to_string() };
        Ok(Response::new(response))
    }

    async fn unregister(
        &self,
        _request: Request<UnregisterRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        let provider = self.avs_provider.read().await;
        // TODO: ask about storing 'config' in the provider
        let config = IvyConfig::load_from_default_path().map_err(IvyError::from)?;
        provider.unregister(&config).await?;

        // TODO: Opt-out flow
        let response = RpcResponse { response_type: 0, msg: "Unregister success.".to_string() };
        Ok(Response::new(response))
    }

    // TODO: Running check stop
    // TODO: On bad netowork, don't change
    async fn select_avs(
        &self,
        request: Request<SelectAvsRequest>,
    ) -> Result<Response<RpcResponse>, Status> {
        // TODO: Clean this up if possible, complexity comes from needing to synchronize the rpc +
        // chain id between provider, signer, and AVS instance.
        let req = request.into_inner();
        let (avs, chain) = (req.avs, try_parse_chain(&req.chain)?);

        {
            let mut provider = self.avs_provider.write().await;
            let signer = provider.provider.signer().clone();
            let config = IvyConfig::load_from_default_path().map_err(IvyError::from)?; // TODO: store config with provider

            let new_ivy_provider =
                connect_provider(&config.get_rpc_url(chain)?, Some(signer)).await?;

            let avs_instance: Box<dyn AvsVariant> = match avs.as_ref() {
                "eigenda" => Box::new(EigenDA::new_from_chain(chain)),
                "altlayer" => Box::new(AltLayer::new_from_chain(chain)),
                _ => return Err(IvyError::InvalidAvsType(avs.to_string()).into()),
            };
            provider.set_avs(avs_instance, new_ivy_provider.into()).await?;
        }

        let response =
            RpcResponse { response_type: 0, msg: format!("AVS set: {} on chain {}", avs, chain) };
        Ok(Response::new(response))
    }
}

#[tonic::async_trait]
impl Operator for IvynetService {
    async fn get_operator_details(
        &self,
        _request: Request<OperatorDetailsRequest>,
    ) -> Result<Response<OperatorDetailsResponse>, Status> {
        let provider = self.avs_provider.read().await;
        let operator_address = provider.provider.address();

        // TODO: parallelize
        let is_registered =
            provider.delegation_manager.is_operator(operator_address).await.map_err(|e| {
                Status::internal(format!("Failed to check if operator is registered: {}", e))
            })?;

        let OperatorDetails {
            earnings_receiver,
            delegation_approver,
            staker_opt_out_window_blocks,
        } = provider
            .delegation_manager
            .operator_details(operator_address)
            .await
            .map_err(|e| Status::internal(format!("Failed to get operator details: {}", e)))?;

        let response = Response::new(OperatorDetailsResponse {
            operator: format!("{:?}", provider.provider.address()),
            is_registered,
            deprecated_earnings_receiver: format!("{:?}", earnings_receiver),
            delegation_approver: format!("{:?}", delegation_approver),
            staker_opt_out_window_blocks,
        });
        Ok(response)
    }

    async fn get_operator_shares(
        &self,
        _request: Request<OperatorSharesRequest>,
    ) -> Result<Response<OperatorSharesResponse>, Status> {
        let provider = self.avs_provider.read().await;
        let operator_address = provider.provider.address();
        let manager = &provider.delegation_manager;
        let strategies = manager.all_strategies().map_err(|e| {
            Status::internal(format!("Failed to get all strategies for operator shares: {}", e))
        })?;
        let shares = manager
            .get_operator_shares(operator_address, strategies.clone())
            .await
            .map_err(|e| Status::internal(format!("Failed to get operator shares: {}", e)))?;
        let operator_shares: Vec<Shares> = zip(strategies.iter(), shares.iter())
            .map(|(s, sh)| Shares { strategy: format!("{:?}", s), shares: sh.to_string() })
            .collect();
        let response = Response::new(OperatorSharesResponse { operator_shares });
        Ok(response)
    }

    async fn get_delegatable_shares(
        &self,
        _request: Request<DelegatableSharesRequest>,
    ) -> Result<Response<DelegatableSharesResponse>, Status> {
        let provider = self.avs_provider.read().await;
        let operator_address = provider.provider.address();
        let manager = &provider.delegation_manager;
        let (strategies, shares) = manager
            .get_delegatable_shares(operator_address)
            .await
            .map_err(|e| Status::internal(format!("Failed to get delegatable shares: {}", e)))?;
        let delegatable_shares: Vec<Shares> = zip(strategies.iter(), shares.iter())
            .map(|(s, sh)| Shares { strategy: format!("{:?}", s), shares: sh.to_string() })
            .collect();
        Ok(Response::new(DelegatableSharesResponse { delegatable_shares }))
    }

    async fn set_ecdsa_keyfile_path(
        &self,
        request: Request<SetEcdsaKeyfilePathRequest>,
    ) -> Result<Response<SetEcdsaKeyfilePathResponse>, Status> {
        let mut provider = self.avs_provider.write().await;
        if let Some(avs) = &provider.avs {
            if avs.running() {
                return Err(Status::failed_precondition("AVS must be stopped to set keyfile path"));
            }
        }

        let req = request.into_inner();
        let path = req.keyfile_path;
        let pass = req.keyfile_password;

        let signer = IvyWallet::from_keystore(path.clone().into(), &pass)?;

        // Update provider
        provider.with_signer(signer)?;
        provider.with_keyfile_pw(Some(pass))?;

        // Update config file
        let mut config = IvyConfig::load_from_default_path().map_err(IvyError::from)?;
        config.default_ecdsa_keyfile = path.into();
        config.store().map_err(IvyError::from)?;

        Ok(Response::new(SetEcdsaKeyfilePathResponse {}))
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
