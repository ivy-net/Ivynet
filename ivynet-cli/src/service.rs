use clap::Subcommand;
use dialoguer::Password;
use ivynet_core::{
    avs::instance::AvsInstance,
    config::IvyConfig,
    error::IvyError,
    grpc::{
        server::Server,
        tonic::{self, Request, Response, Status},
    },
    server::build_avs_provider,
    wallet::IvyWallet,
};
use ivynet_core::{
    avs::{AvsProvider, AvsVariant},
    grpc::ivynet_api::ivy_daemon::{
        ivy_daemon_server::{IvyDaemon, IvyDaemonServer},
        InfoRequest, InfoResponse, RpcResponse,
    },
    grpc::ivynet_api::ivy_daemon_avs::Stop,
    grpc::ivynet_api::ivy_daemon_types::ResponseType,
};
use tokio::sync::{oneshot, Mutex, RwLock};

use crate::error::Error;

const DEFAULT_PORT: u16 = 55501;

#[derive(Debug, Subcommand)]
pub enum ServiceCommands {
    #[command(
        name = "serve",
        about = "Start the Ivynet service with a specified AVS on a specified chain. <AVS> <CHAIN> [PORT]"
    )]
    Serve { avs: String, chain: String, port: Option<u16> },
}

#[derive(Debug)]
pub struct IvyDaemonService<T: AvsVariant> {
    /// The AVS provider instance that was used to initialize the AVS and manage the Ivynet
    /// service
    avs: RwLock<AvsProvider<T>>,
    /// Handle to the spawned process containing the AVS, always a Docker container
    process: std::process::Child,
    /// Message sender for the purpose of stopping the server or other top-level actions
    shutdown_sender: Mutex<Option<oneshot::Sender<()>>>,
}

#[tonic::async_trait]
impl IvyDaemon for IvyDaemonService<AvsInstance> {
    // TODO: Dummy implementation, replace with actual AVS info
    async fn get_info(
        &self,
        request: tonic::Request<InfoRequest>,
    ) -> Result<tonic::Response<InfoResponse>, tonic::Status> {
        let response = self.avs.read().await.get_bootable_quorums().await;
        let response = "Okay :)".to_string();
        let reply = InfoResponse { avs_name: response };
        Ok(Response::new(reply))
    }

    async fn stop(&self, _request: Request<Stop>) -> Result<Response<RpcResponse>, Status> {
        let mut sender = self.shutdown_sender.lock().await;
        let avs = self.avs.write().await;
        let chain = avs.chain().await.expect("Could not get chain");
        avs.stop(chain).await.expect("Could not stop AVS service");
        if let Some(sender) = sender.take() {
            if sender.send(()).is_ok() {
                let response = RpcResponse { response_type: 0, msg: "Server is shutting down".to_string() };
                // TODO: Create kill flow
                Ok(Response::new(response))
            } else {
                Err(Status::internal("Failed to send shutdown signal"))
            }
        } else {
            Err(Status::internal("Shutdown signal already sent or sender not available"))
        }
    }
}

pub async fn serve(avs: String, chain: String, port: Option<u16>, config: &IvyConfig) -> Result<(), Error> {
    let port = port.unwrap_or(DEFAULT_PORT);

    let password: String = Password::new().with_prompt("Input the password for your stored keyfile").interact()?;
    let wallet = IvyWallet::from_keystore(config.default_private_keyfile.clone(), password)?;
    let avs_provider = build_avs_provider(&avs, &chain, config, Some(wallet)).await?;

    println!("Starting AVS: {avs}");
    let process = avs_provider.start(config).await?;
    let (shutdown_sender, shutdown_receiver) = oneshot::channel::<()>();
    let sender = Mutex::new(Some(shutdown_sender));

    let service = IvyDaemonService { avs: RwLock::new(avs_provider), process, shutdown_sender: sender };
    let server = Server::new(IvyDaemonServer::new(service), None, None);

    let serve_future = server.serve_with_shutdown(port, shutdown_receiver);

    println!("Starting the IvyNet service on port {}...", port);
    serve_future.await.expect("Failed to start IvyNet Service.");
    Ok(())
}

// TODO: Entire flow for kill method goes here. Return a stream of responses as kill steps
// progress.
pub fn kill_flow(pid: i32) -> Result<(), IvyError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{config, operator, staker};
    use clap::{ArgMatches, Parser, Subcommand};
    use ivynet_core::avs::commands::AvsCommands;

    #[derive(Parser, Debug)]
    #[command(name = "ivy", version, about = "The command line interface for ivynet")]
    struct Args {
        #[command(subcommand)]
        cmd: Commands,

        /// The network to connect to: mainnet, holesky, local
        #[arg(long, short, default_value = "holesky")]
        network: String,
    }

    #[derive(Subcommand, Debug)]
    enum Commands {
        #[command(name = "init", about = "Ivynet config intiliazation")]
        Init,
        #[command(name = "avs", about = "Request information about an AVS or boot up a node")]
        Avs {
            #[command(subcommand)]
            subcmd: AvsCommands,
        },
        #[command(name = "config", about = "Manage rpc information, keys, and keyfile settings")]
        Config {
            #[command(subcommand)]
            subcmd: config::ConfigCommands,
        },

        #[command(name = "operator", about = "Request information, register, or manage your operator")]
        Operator {
            #[command(subcommand)]
            subcmd: operator::OperatorCommands,
        },
        #[command(name = "staker", about = "Request information about stakers")]
        Staker {
            #[command(subcommand)]
            subcmd: staker::StakerCommands,
        },

        #[command(
            name = "serve",
            about = "Start the Ivynet service with a specified AVS on a specified chain. <AVS> <CHAIN> [PORT]"
        )]
        Serve { avs: String, chain: String, port: Option<u16> },
    }

    #[test]
    fn test_parse_args() {
        let args_vec: Vec<&str> = vec!["ivy", "serve", "eigenda", "holesky"];
        let args = Args::parse_from(args_vec);
        println!("{:?}", args);
    }
}
