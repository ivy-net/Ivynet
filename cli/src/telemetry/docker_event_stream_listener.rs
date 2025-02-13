use std::time::Duration;

use bollard::secret::{EventMessage, EventMessageTypeEnum};
use ivynet_docker::{
    container::{ContainerId, ContainerImage},
    dockerapi::{DockerApi, DockerClient, DockerStreamError},
};
use ivynet_grpc::{
    backend::backend_client::BackendClient,
    messages::{NodeDataV2, NodeTypeQueries, NodeTypeQuery},
    tonic::{transport::Channel, Request, Response},
    BackendMiddleware,
};
use tokio::time::sleep;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};

use crate::ivy_machine::{IvyMachine, MachineIdentityError};

use super::{
    dispatch::{TelemetryDispatchError, TelemetryDispatchHandle},
    logs_listener::LogsListenerManager,
    machine_data_listener::MachineDataMonitorHandle,
    metrics_listener::MetricsListenerHandle,
    node_data_listener::NodeDataMonitorHandle,
    ConfiguredAvs,
};

const TELEMETRY_INTERVAL_IN_MINUTES: u64 = 1;

#[derive(Debug)]
pub struct DockerStreamListener<D: DockerApi, B: BackendMiddleware> {
    pub docker: D,
    pub machine_data_monitor_handle: MachineDataMonitorHandle<B>,
    pub node_data_monitor_handle: NodeDataMonitorHandle<B>,
    pub metrics_listener_handle: MetricsListenerHandle,
    pub logs_listener_handle: LogsListenerManager,
    pub dispatch: TelemetryDispatchHandle,
    pub machine: IvyMachine,
    pub backend: BackendClient<Channel>,
}

impl<B: BackendMiddleware> DockerStreamListener<DockerClient, B> {
    pub fn new(
        machine_data_monitor: MachineDataMonitorHandle<B>,
        node_data_monitor: NodeDataMonitorHandle<B>,
        metrics_listener: MetricsListenerHandle,
        logs_listener: LogsListenerManager,
        dispatch: TelemetryDispatchHandle,
        machine: IvyMachine,
        backend: BackendClient<Channel>,
    ) -> Self {
        Self {
            docker: DockerClient::default(),
            machine_data_monitor_handle: machine_data_monitor,
            node_data_monitor_handle: node_data_monitor,
            metrics_listener_handle: metrics_listener,
            logs_listener_handle: logs_listener,
            dispatch,
            machine,
            backend,
        }
    }

    pub async fn run(
        mut self,
        known_nodes: Vec<ConfiguredAvs>,
    ) -> Result<(), DockerStreamListenerError> {
        let mut docker_stream = self.docker.stream_events().await;

        let mut telemetry_interval =
            tokio::time::interval(Duration::from_secs(TELEMETRY_INTERVAL_IN_MINUTES * 60));

        loop {
            tokio::select! {
                // 1) A Docker event arrives.
                maybe_event = docker_stream.next() => {
                    match maybe_event {
                        Some(Ok(event)) => {
                            debug!("Dockerstream Event | {:?}", event);
                            if event.typ == Some(EventMessageTypeEnum::CONTAINER) {
                                if let Some(action) = event.action.as_deref() {
                                    match action {
                                        "start" => {
                                            self.on_start(event, &known_nodes).await?;
                                        }
                                        "stop" | "kill" | "die" => {
                                            self.on_stop(event).await?;
                                        }
                                        _ => {
                                        }
                                    }
                                }
                            }
                        },
                        Some(Err(err)) => {
                            error!("Dockerstream Error | {:?}", err);
                        },
                        None => {
                            // The Docker event stream ended.
                            break;
                        }
                    }
                }

                // 2) The telemetry interval ticks.
                _ = telemetry_interval.tick() => {
                    info!("Broadcasting telemetry on tick...");

                    let signed_machine_data = self.machine.sign_machine_data()?;
                    if let Err(e) = self.machine_data_monitor_handle.ask_send_machine_data(signed_machine_data).await {
                        error!("Failed to send machine data: {}", e);
                    }

                    for node in known_nodes.iter() {
                        let manifest = node.manifest.clone().unwrap_or(ContainerId("".to_string()));
                        let node_data = NodeDataV2 {
                            name: node.assigned_name.to_string(),
                            node_type: Some(node.avs_type.clone()),
                            manifest: Some(manifest.to_string()),
                            metrics_alive: Some(node.metrics_alive().await),
                            node_running: Some(node.node_running().await),
                        };
                        let signed = self.machine.sign_node_data_v2(&node_data)?;
                        if let Err(e) = self.node_data_monitor_handle.ask_send_node_data(signed).await {
                            error!("Failed to send node data: {}", e);
                        }
                    }

                    // Broadcast telemetry to your metrics listener
                    if let Err(e) = self.metrics_listener_handle.tell_broadcast().await{
                        error!("Error broadcasting metrics: {:?}", e);
                        };


                }
            }
        }

        Ok(())
    }

    pub async fn on_start(
        &mut self,
        event: EventMessage,
        avses: &[ConfiguredAvs],
    ) -> Result<(), DockerStreamListenerError> {
        let actor = event.actor.ok_or(DockerStreamError::MissingActor)?;
        let attributes = actor.attributes.ok_or(DockerStreamError::MissingAttributes)?;
        let inc_container_name =
            attributes.get("name").ok_or(DockerStreamError::MissingAttributes)?;

        let inc_container = match self.docker.find_container_by_name(inc_container_name).await {
            Some(container) => container,
            None => {
                return Ok(());
            }
        };

        let inc_image_name = inc_container.image().unwrap_or_default().to_string();
        let inc_container_digest = inc_container.image_id().unwrap_or_default().to_string();

        let metrics_port = match inc_container.metrics_port(&self.docker).await {
            Some(port) => Some(port),
            None => {
                // wait for metrics port to potentially come up
                sleep(Duration::from_secs(10)).await;
                inc_container.metrics_port(&self.docker).await
            }
        };

        let mut configured = None;
        for avs in avses {
            // First try to find by container name
            if avs.container_name == *inc_container_name {
                configured = Some(avs.clone());
                break;
            }
            // If not found by name, check if any existing AVS is monitoring
            // the same container (by image hash)
            if let Some(existing_container) =
                self.docker.find_container_by_name(&avs.container_name).await
            {
                if let Some(existing_digest) = existing_container.image_id() {
                    if existing_digest == inc_container_digest {
                        configured = Some(avs.clone());
                        break;
                    }
                }
            }
        }

        let configured = match configured {
            Some(avs) => Some(avs),
            None => {
                let node_type_query = NodeTypeQuery {
                    container_name: inc_container_name.clone(),
                    image_name: inc_image_name.clone(),
                    image_digest: inc_container_digest.clone(),
                };
                let query = Request::new(NodeTypeQueries { node_types: vec![node_type_query] });

                let response =
                    self.backend.node_type_queries(query).await.map(Response::into_inner).ok();

                // Only create configuration if we get a valid node type
                match response {
                    Some(node_type) => {
                        if let Some(node_type) = node_type.node_types.first() {
                            if node_type.node_type != "unknown" {
                                Some(ConfiguredAvs {
                                    assigned_name: inc_container_name.to_string(),
                                    container_name: inc_container_name.clone(),
                                    avs_type: node_type.node_type.clone(),
                                    metric_port: metrics_port,
                                    manifest: Some(ContainerId::from(
                                        inc_container_digest.as_str(),
                                    )),
                                    image: Some(ContainerImage::from(inc_image_name.as_str())),
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            }
        };

        if let Some(configured) = configured {
            debug!("Found container: {}", inc_container_name);

            let node_data_v2 = NodeDataV2 {
                name: configured.assigned_name.clone(),
                node_type: Some(configured.avs_type.clone()),
                manifest: Some(inc_container_digest.clone()),
                metrics_alive: Some(configured.metrics_alive().await),
                node_running: Some(true),
            };
            let signed = self.machine.sign_node_data_v2(&node_data_v2)?;

            if let Err(e) = self.node_data_monitor_handle.ask_send_node_data(signed).await {
                error!("Error sending node data: {:?}", e);
            }
            if let Err(e) = self.metrics_listener_handle.tell_add_node(configured.clone()).await {
                error!("Error adding node: {:?}", e);
            }
            if let Err(e) =
                self.logs_listener_handle.add_listener(&inc_container, &configured).await
            {
                error!("Error adding listener: {:?}", e);
            }
        }

        Ok(())
    }

    pub async fn on_stop(&self, event: EventMessage) -> Result<(), DockerStreamError> {
        let actor = event.actor.ok_or(DockerStreamError::MissingActor)?;
        let attributes = actor.attributes.ok_or(DockerStreamError::MissingAttributes)?;
        let container_name = attributes.get("name").ok_or(DockerStreamError::MissingAttributes)?;

        debug!("Container stopped: {}", container_name);

        if let Err(e) =
            self.metrics_listener_handle.tell_remove_node_by_name(container_name.to_string()).await
        {
            error!("Error removing node: {:?}", e);
        };

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DockerStreamListenerError {
    #[error("Dockerstream error: {0}")]
    DockerStreamError(#[from] DockerStreamError),

    #[error("Ivynet signing error: {0}")]
    SigningError(#[from] ivynet_signer::sign_utils::IvySigningError),

    #[error("Telemetry dispatch error: {0}")]
    DispatchError(#[from] TelemetryDispatchError),

    #[error("Machine identity error: {0}")]
    MachineIdentityError(#[from] MachineIdentityError),
}
