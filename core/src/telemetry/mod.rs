use crate::{
    error::IvyError,
    grpc::{backend::backend_client::BackendClient, tonic::transport::Channel},
    wallet::IvyWallet,
};
use bollard::secret::{EventMessage, EventMessageTypeEnum};
use dispatch::{TelemetryDispatchError, TelemetryDispatchHandle};
use ivynet_docker::{dockerapi::{DockerClient, DockerStreamError}, eventstream::DockerEventHandler};
use ivynet_node_type::NodeType;
use logs_listener::{ListenerData, LogsListenerManager};
use metrics_listener::MetricsListenerHandle;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::broadcast,
    time::{sleep, Duration},
};
use tokio_stream::StreamExt;
use tracing::{error, warn};
use uuid::Uuid;

pub mod dispatch;
pub mod logs_listener;
pub mod metrics_listener;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfiguredAvs {
    pub assigned_name: String,
    pub container_name: String,
    pub avs_type: NodeType,
    pub metric_port: Option<u16>,
}

pub async fn listen(
    backend_client: BackendClient<Channel>,
    machine_id: Uuid,
    identity_wallet: IvyWallet,
    avses: &[ConfiguredAvs],
) -> Result<(), IvyError> {
    let dispatch = TelemetryDispatchHandle::from_client(backend_client).await;
    let error_rx = dispatch.error_rx.resubscribe();
    let docker = DockerClient::default();

    // The logs listener spawns the future immediately and does not need to be awaited with
    // tokio::select!
    let mut logs_listener = LogsListenerManager::new(dispatch.clone(), docker.clone());

    // Metrics listener errors no longer cause the program to stop, but instead throw an error msg
    // up to console.
    let metrics_listener_handle =
        MetricsListenerHandle::new(machine_id, &identity_wallet, avses, &dispatch);

    let mut docker_stream = docker.stream_events();
    while let Some(event) = docker_stream.next().await {
        match event {
            Ok(event) => {
                if let Some(EventMessageTypeEnum::CONTAINER) = event.typ {
                    if let Some(action) = event.action.as_deref() {
                        match action {
                            "start" => {
                                let actor= event.actor.ok_or(DockerStreamError::MissingActor)?;
                                let attributes = actor.attributes.ok_or(DockerStreamError::MissingAttributes)?;
                                let container_name = attributes.get("name").ok_or(DockerStreamError::MissingAttributes)?;
                                
                                let image = attributes.get("image").ok_or(DockerStreamError::MissingAttributes)?;

                                let container = match docker.find_container_by_name(container_name).await {
                                    Some(container) => container,
                                    None => {
                                        continue;
                                    }
                                };

                                let metrics_port = match container.metrics_port().await {
                                    Some(port) => port,
                                    None => {
                                        // wait for metrics port to potentially come up
                                        sleep(Duration::from_secs(10)).await;
                                        match container.metrics_port().await {
                                            Some(port) => port,
                                            None => {
                                                continue;
                                            }
                                        }
                                    }
                                };

                                let configured = match avses.iter().find(|avs| avs.container_name == *container_name) {
                                    Some(avs) => avs.clone(),
                                    None => {
                                        ConfiguredAvs {
                                            assigned_name: "unknown".to_owned(),
                                            container_name: container_name.clone(),
                                            avs_type: NodeType::Unknown,
                                            metric_port: Some(metrics_port),
                                        
                                        }
                                    }
                                };

                                metrics_listener_handle.add_node(configured).await;


                            }
                            "stop" | "kill" | "die" => {
                                println!("Container stopped: {:?}", event);
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Error in docker event stream: {:?}", e);
            }
        }
    }

    tokio::select! {
        // metrics_err = listen_metrics(machine_id, &identity_wallet, avses, &dispatch) => {
        //     match metrics_err {
        //         Ok(_) => unreachable!("Metrics listener should never return Ok"),
        //         Err(err) => {
        //             error!("Cannot listen for metrics ({err:?})");
        //             Err(err)
        //         },
        //     }
        // }
        _ = handle_telemetry_errors(error_rx) => Ok(())
    }
}

async fn handle_docker_event(event: EventMessage) {
    let 
}

async fn handle_telemetry_errors(mut error_rx: broadcast::Receiver<TelemetryDispatchError>) {
    while let Ok(error) = error_rx.recv().await {
        error!("Received telemetry error: {}", error);
        sleep(Duration::from_secs(30)).await;
    }
}
