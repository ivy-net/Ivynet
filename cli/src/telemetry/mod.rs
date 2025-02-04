use std::{collections::HashMap, sync::Arc};

use convert_case::{Case, Casing};
use dispatch::{TelemetryDispatchError, TelemetryDispatchHandle};
use docker_event_stream_listener::DockerStreamListener;
use ivynet_docker::{
    container::{ContainerId, ContainerImage},
    dockerapi::{DockerApi, DockerClient},
};
use ivynet_grpc::{backend::backend_client::BackendClient, tonic::transport::Channel};
use logs_listener::LogsListenerManager;
use metrics_listener::MetricsListenerHandle;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use crate::{error::Error, ivy_machine::IvyMachine};

pub mod dispatch;
pub mod docker_event_stream_listener;
pub mod logs_listener;
pub mod metrics_listener;
pub mod parser;

pub type ErrorChannelTx = broadcast::Sender<TelemetryError>;
pub type ErrorChannelRx = broadcast::Receiver<TelemetryError>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum TelemetryError {
    #[error("Telemetry dispatch error: {0}")]
    DispatchError(TelemetryDispatchError),

    #[error("Docker stream error: {0}")]
    DockerStreamError(ivynet_docker::dockerapi::DockerStreamError),

    #[error("Telemetry dispatch error: {0}")]
    TelemetryDispatchError(#[from] TelemetryDispatchError),

    #[error("Metrics listener error: {0}")]
    MetricsListenerError(Arc<metrics_listener::MetricsListenerError>),
}

#[derive(Clone, Debug, Serialize, Hash, Eq, PartialEq)]
pub struct ConfiguredAvs {
    pub assigned_name: String,
    pub container_name: String,
    pub avs_type: String,
    pub metric_port: Option<u16>,
    pub manifest: Option<ContainerId>,
    pub image: Option<ContainerImage>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum AvsTypeField {
    Simple(String),
    Compound(HashMap<String, String>),
}

impl<'de> Deserialize<'de> for ConfiguredAvs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Helper {
            assigned_name: String,
            container_name: String,
            #[serde(default)]
            metric_port: Option<u16>,
            avs_type: AvsTypeField,
            image: Option<ContainerImage>,
            manifest: Option<ContainerId>,
        }

        let helper = Helper::deserialize(deserializer)?;

        let avs_type = match helper.avs_type {
            AvsTypeField::Simple(s) => s,
            AvsTypeField::Compound(map) => {
                let (key, value) = map
                    .into_iter()
                    .next()
                    .ok_or_else(|| serde::de::Error::custom("Empty compound type"))?;

                // Convert outer to lowercase (consistent with NodeType)
                let outer = key.to_case(Case::Kebab);

                // Convert inner directly to kebab case (matching NodeType behavior)
                let inner = value.to_case(Case::Kebab);

                format!("{}({})", outer, inner)
            }
        };

        Ok(ConfiguredAvs {
            assigned_name: helper.assigned_name,
            container_name: helper.container_name.trim_start_matches('/').to_string(),
            avs_type,
            metric_port: helper.metric_port,
            manifest: helper.manifest,
            image: helper.image,
        })
    }
}

/**
 * -------------NODE LISTENER COMPOSITION-------------
 * The listen function is the entry point for the telemetry module. It is responsible for
 * setting up the various listeners and dispatchers that will handle telemetry data.
 *
 * The listen function initializes and composes the following actors:
 *
 * 1. Dispatcher: The dispatcher is responsible for receiving telemetry data from the various
 *    other listeners via a tokio mpsc channel and sending it to the backend. It is the central
 *    hub for telemetry data transmission. Interface is accessible via the
 *    TelemetryDispatchHandle.
 *
 * 2. Logs Listener: The logs listener is responsible for listening to logs from containers and
 *    sending them to the dispatcher. It is composed of a LogsListenerManager and a set of
 *    LogsListeners. The LogsListenerManager is responsible for managing the set of listeners
 *    and spawning them as futures. The LogsListeners are responsible for listening to logs from
 *    a single container and sending them to the dispatcher. If a given LogsListener receives a
 *    signal that the docker log stream is closed, it shuts down and is removed from the managed
 *    list. The LogsListenerManager serves as the handle for the interior set of all logs
 *    listeners.
 *
 * 3. Metrics Listener: The metrics listener is responsible for listening to metrics from
 *    containers and sending them to the dispatcher. It receives an initial set of configured
 *    nodes and sends metrics for all containers in its set to the dispatcher at fixed
 *    intervals. Additionally, its list of nodes may be managed via the MetricsListenerHandle
 *    interface, and it will transmit metrics for all nodes in its set after each update in
 *    addition to the fixed interval.
 *
 * 4. Docker Stream Listener: The docker stream listener is responsible for listening to docker
 *    stream events and sending them to the other listeners for processing. It has no associated
 *    handle and is spawned as a future in the listen function.
 *
 */
pub async fn listen(
    backend_client: BackendClient<Channel>,
    machine: IvyMachine,
    avses: &[ConfiguredAvs],
) -> Result<(), Error> {
    let docker = DockerClient::default();

    let (error_tx, error_rx) = tokio::sync::broadcast::channel(64);

    // Telemtry dispatcher recieves telemetry messages from other listeners and sends them to the
    // backend
    let dispatch = TelemetryDispatchHandle::new(backend_client.clone(), &error_tx).await;

    // Logs Listener handles logs from containers and sends them to the dispatcher
    let mut logs_listener_handle =
        LogsListenerManager::new(&docker, machine.clone().into(), &dispatch);

    for node in avses {
        info!("Searching for node: {}", node.container_name);
        if let Some(container) = &docker.find_container_by_name(&node.container_name).await {
            if let Err(e) = logs_listener_handle.add_listener(container, node).await {
                error!("Failed to add logs listener for container: {}", e);
            };
        } else {
            warn!("Cannot find container for configured node: {}.", node.container_name);
        }
    }

    // Metrics Listener handles metrics from containers and sends them to the dispatcher
    let metrics_listener_handle =
        MetricsListenerHandle::new(&docker, machine.clone(), avses, &dispatch, error_tx);

    // Stream listener listens for docker events and sends them to the other listeners for
    // processing
    let docker_listener = DockerStreamListener::new(
        metrics_listener_handle,
        logs_listener_handle,
        dispatch.clone(),
        machine,
        backend_client,
    );
    tokio::spawn(docker_listener.run(avses.to_vec()));

    // This should never return unless the error channel is closed
    handle_telemetry_errors(error_rx).await?;

    Ok(())
}

async fn handle_telemetry_errors(mut error_rx: ErrorChannelRx) -> Result<(), Error> {
    while let Ok(error) = error_rx.recv().await {
        error!("Received telemetry error: {}", error);
    }
    Ok(())
}

#[cfg(test)]
mod monitor_config_tests {
    use crate::monitor::MonitorConfig;

    use super::*;
    use serde_json::json;
    use toml;

    #[test]
    fn test_simple_string_format() {
        let json = json!({
            "assigned_name": "test",
            "container_name": "/test",
            "avs_type": "EigenDA",
            "metric_port": 9092
        });

        let avs: ConfiguredAvs = serde_json::from_value(json).unwrap();
        assert_eq!(avs.assigned_name, "test");
        assert_eq!(avs.avs_type, "EigenDA");
        assert_eq!(avs.metric_port, Some(9092));
    }

    #[test]
    fn test_compound_table_format() {
        let json = json!({
            "assigned_name": "test",
            "container_name": "/test",
            "avs_type": {
                "Altlayer": "AltlayerMach"
            }
        });

        let avs: ConfiguredAvs = serde_json::from_value(json).unwrap();
        assert_eq!(avs.avs_type, "altlayer(altlayer-mach)");
    }

    #[test]
    fn test_toml_compatibility() {
        let toml_str = r#"
            [[configured_avses]]
            assigned_name = "eigenda"
            container_name = "/eigenda-native-node"
            avs_type = "EigenDA"
            metric_port = 9092

            [[configured_avses]]
            assigned_name = "altlayer"
            container_name = "/mach-avs"
            [configured_avses.avs_type]
            Altlayer = "AltlayerMach"
        "#;

        let config: MonitorConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.configured_avses.len(), 2);
        assert_eq!(config.configured_avses[0].avs_type, "EigenDA");
        assert_eq!(config.configured_avses[1].avs_type, "altlayer(altlayer-mach)");
    }

    #[test]
    fn test_case_insensitivity() {
        let json = json!({
            "assigned_name": "test",
            "container_name": "/test",
            "avs_type": {
                "ALTLAYER": "altlayermach"
            }
        });

        let avs: ConfiguredAvs = serde_json::from_value(json).unwrap();
        assert_eq!(avs.avs_type, "altlayer(altlayermach)");
    }

    #[test]
    fn test_optional_metric_port() {
        let json = json!({
            "assigned_name": "test",
            "container_name": "/test",
            "avs_type": "EigenDA"
        });

        let avs: ConfiguredAvs = serde_json::from_value(json).unwrap();
        assert_eq!(avs.metric_port, None);
    }

    #[test]
    fn test_error_cases() {
        // Empty compound type
        let json = json!({
            "assigned_name": "test",
            "container_name": "/test",
            "avs_type": {}
        });
        assert!(serde_json::from_value::<ConfiguredAvs>(json).is_err());

        // Missing required fields
        let json = json!({
            "assigned_name": "test",
            "avs_type": "EigenDA"
        });
        assert!(serde_json::from_value::<ConfiguredAvs>(json).is_err());
    }

    #[test]
    fn test_mixed_format_config() {
        let toml_str = r#"
            [[configured_avses]]
            assigned_name = "eigenda"
            container_name = "/eigenda"
            avs_type = "eigenda(native-node)"

            [[configured_avses]]
            assigned_name = "altlayer"
            container_name = "/altlayer"
            [configured_avses.avs_type]
            Altlayer = "AltlayerMach"

            [[configured_avses]]
            assigned_name = "simple"
            container_name = "/simple"
            avs_type = "EigenDA"
        "#;

        let config: MonitorConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.configured_avses.len(), 3);
        assert_eq!(config.configured_avses[0].avs_type, "eigenda(native-node)");
        assert_eq!(config.configured_avses[1].avs_type, "altlayer(altlayer-mach)");
        assert_eq!(config.configured_avses[2].avs_type, "EigenDA");
    }

    #[test]
    fn test_unknown_node_type() {
        let json = json!({
            "assigned_name": "test",
            "container_name": "/test",
            "avs_type": "UnknownType"
        });

        let avs: ConfiguredAvs = serde_json::from_value(json).unwrap();
        assert_eq!(avs.avs_type, "UnknownType");
    }

    #[test]
    fn test_compound_format_variations() {
        let variations = vec![
            (
                json!({
                    "Altlayer": "altlayer-mach"
                }),
                "altlayer(altlayer-mach)",
            ),
            (
                json!({
                    "UngateInfiniRoute": "UnknownL2"
                }),
                "ungate-infini-route(unknown-l-2)",
            ),
            (
                json!({
                    "SkateChain": "Base"
                }),
                "skate-chain(base)",
            ),
        ];

        for (input, expected) in variations {
            let json = json!({
                "assigned_name": "test",
                "container_name": "/test",
                "avs_type": input
            });

            let avs: ConfiguredAvs = serde_json::from_value(json).unwrap();
            assert_eq!(avs.avs_type, expected);
        }
    }

    #[test]
    fn test_container_name_slash_handling() {
        let variations = vec![
            ("/test", "test"),
            ("///test", "test"),
            ("test", "test"),
            ("/test/nested", "test/nested"),
        ];

        for (input, expected) in variations {
            let json = json!({
                "assigned_name": "test",
                "container_name": input,
                "avs_type": "EigenDA"
            });

            let avs: ConfiguredAvs = serde_json::from_value(json).unwrap();
            assert_eq!(avs.container_name, expected);
        }
    }
}
