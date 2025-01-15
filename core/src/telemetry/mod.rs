use std::{collections::HashMap, time::Duration};

use crate::{
    error::IvyError,
    grpc::{
        backend::backend_client::BackendClient,
        messages::{Metrics, MetricsAttribute, NodeData, SignedMetrics, SignedNodeData},
        tonic::transport::Channel,
    },
    signature::{sign_metrics, sign_node_data},
    system::get_detailed_system_information,
    wallet::IvyWallet,
};
use dispatch::{TelemetryDispatchError, TelemetryDispatchHandle};
use docker_event_stream_listener::DockerStreamListener;
use ivynet_docker::dockerapi::{DockerApi, DockerClient};
use ivynet_node_type::NodeType;
use logs_listener::LogsListenerManager;
use metrics_listener::MetricsListenerHandle;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{sync::broadcast, time::sleep};
use tracing::{error, info, warn};
use uuid::Uuid;

pub mod dispatch;
pub mod docker_event_stream_listener;
pub mod logs_listener;
pub mod metrics_listener;
pub mod parser;

pub type ErrorChannelTx = broadcast::Sender<TelemetryError>;
pub type ErrorChannelRx = broadcast::Receiver<TelemetryError>;

#[derive(Clone, Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("Telemetry dispatch error: {0}")]
    DispatchError(TelemetryDispatchError),

    #[error("Docker stream error: {0}")]
    DockerStreamError(ivynet_docker::dockerapi::DockerStreamError),

    #[error("Telemetry dispatch error: {0}")]
    TelemetryDispatchError(#[from] TelemetryDispatchError),

    #[error("Metrics listener error: {0}")]
    MetricsListenerError(#[from] metrics_listener::MetricsListenerError),
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct ConfiguredAvs {
    pub assigned_name: String,
    pub container_name: String,
    pub avs_type: NodeType,
    pub metric_port: Option<u16>,
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
    machine_id: Uuid,
    identity_wallet: IvyWallet,
    avses: &[ConfiguredAvs],
) -> Result<(), IvyError> {
    let docker = DockerClient::default();

    let (error_tx, error_rx) = tokio::sync::broadcast::channel(64);

    // Telemtry dispatcher recieves telemetry messages from other listeners and sends them to the
    // backend
    let dispatch = TelemetryDispatchHandle::new(backend_client.clone(), &error_tx).await;

    // Logs Listener handles logs from containers and sends them to the dispatcher
    let mut logs_listener_handle =
        LogsListenerManager::new(&docker, &identity_wallet, machine_id, &dispatch);

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
    let metrics_listener_handle = MetricsListenerHandle::new(
        &docker,
        machine_id,
        &identity_wallet,
        avses,
        &dispatch,
        error_tx,
    );

    // Stream listener listens for docker events and sends them to the other listeners for
    // processing
    let docker_listener =
        DockerStreamListener::new(metrics_listener_handle, logs_listener_handle, backend_client);
    tokio::spawn(docker_listener.run(avses.to_vec()));

    // This should never return unless the error channel is closed
    handle_telemetry_errors(error_rx).await?;

    Ok(())
}

async fn handle_telemetry_errors(mut error_rx: ErrorChannelRx) -> Result<(), IvyError> {
    while let Ok(error) = error_rx.recv().await {
        error!("Received telemetry error: {}", error);
        sleep(Duration::from_secs(30)).await;
    }
    Ok(())
}

/// This function is responsible for broadcasting both metrics (if any are found for a configured
/// node) as well as the node data for that configured node. On the receiver side, if an empty
/// metrics vector is send, we know that metrics are not accessible for the node and we mark it
/// as such. We can make this explicit in the future.
pub async fn listen_metrics(
    machine_id: Uuid,
    identity_wallet: &IvyWallet,
    avses: &[ConfiguredAvs],
    dispatch: &TelemetryDispatchHandle,
) -> Result<(), IvyError> {
    let docker = DockerClient::default();
    let mut node_types = HashMap::new();
    let mut prev_version_hashes = HashMap::new();
    let mut are_running = HashMap::new();

    for avs in avses {
        node_types.insert(avs, Some(avs.avs_type.to_string()));
        prev_version_hashes.insert(avs, "".to_string());
        are_running.insert(avs, false);
    }
    loop {
        let images = docker.list_images().await;
        for avs in avses {
            let mut version_hash = "".to_string();
            if let Some(inspect_data) = docker.find_container_by_name(&avs.container_name).await {
                if let Some(image_name) = inspect_data.image() {
                    if let Some(hash) = images.get(image_name) {
                        version_hash = hash.clone();
                    }
                }
            }

            let metrics = if let Some(port) = avs.metric_port {
                let metrics: Vec<Metrics> = fetch_telemetry_from(port).await.unwrap_or_default();

                let metrics_signature = sign_metrics(metrics.as_slice(), identity_wallet)?;
                let signed_metrics = SignedMetrics {
                    machine_id: machine_id.into(),
                    avs_name: Some(avs.assigned_name.clone()),
                    signature: metrics_signature.to_vec(),
                    metrics: metrics.to_vec(),
                };

                dispatch.send_metrics(signed_metrics).await?;

                metrics
            } else {
                Vec::new()
            };

            let is_running = docker.is_running(&avs.container_name).await;

            // Send node data
            let node_data = NodeData {
                name: avs.assigned_name.to_string(),
                node_type: node_types[avs].clone(),
                manifest: if prev_version_hashes[avs] == version_hash {
                    None
                } else {
                    Some(version_hash.clone())
                },
                metrics_alive: Some(!metrics.is_empty()),
                node_running: if is_running != are_running[avs] { Some(true) } else { None },
            };

            let node_data_signature = sign_node_data(&node_data, identity_wallet)?;
            let signed_node_data = SignedNodeData {
                machine_id: machine_id.into(),
                signature: node_data_signature.to_vec(),
                node_data: Some(node_data),
            };

            dispatch.send_node_data(signed_node_data).await?;
            node_types.insert(avs, None);
            prev_version_hashes.insert(avs, version_hash);
            are_running.insert(avs, is_running);
        }
        // Last but not least - send system metrics
        if let Ok(system_metrics) = fetch_system_telemetry().await {
            let metrics_signature = sign_metrics(system_metrics.as_slice(), identity_wallet)?;
            let signed_metrics = SignedMetrics {
                machine_id: machine_id.into(),
                avs_name: None,
                signature: metrics_signature.to_vec(),
                metrics: system_metrics.to_vec(),
            };
            dispatch.send_metrics(signed_metrics).await?;
        }

        // Construct and send node data

        sleep(Duration::from_secs(60)).await;
    }
}

pub async fn fetch_telemetry_from(port: u16) -> Result<Vec<Metrics>, IvyError> {
    let client = Client::new();
    if let Ok(resp) = client
        .get(format!("http://localhost:{}/metrics", port))
        .timeout(Duration::from_secs(10))
        .send()
        .await
    {
        if let Ok(body) = resp.text().await {
            let metrics = body
                .split('\n')
                .filter_map(|line| TelemetryParser::new(line).parse())
                .collect::<Vec<_>>();

            Ok(metrics)
        } else {
            Err(IvyError::NotFound)
        }
    } else {
        Err(IvyError::NotFound)
    }
}

async fn fetch_system_telemetry() -> Result<Vec<Metrics>, IvyError> {
    // Now we need to add basic metrics
    let (cores, cpu_usage, ram_usage, free_ram, disk_usage, free_disk, uptime) =
        get_detailed_system_information();

    Ok(vec![
        Metrics { name: "cpu_usage".to_owned(), value: cpu_usage, attributes: Default::default() },
        Metrics {
            name: "ram_usage".to_owned(),
            value: ram_usage as f64,
            attributes: Default::default(),
        },
        Metrics {
            name: "free_ram".to_owned(),
            value: free_ram as f64,
            attributes: Default::default(),
        },
        Metrics {
            name: "disk_usage".to_owned(),
            value: disk_usage as f64,
            attributes: Default::default(),
        },
        Metrics {
            name: "free_disk".to_owned(),
            value: free_disk as f64,
            attributes: Default::default(),
        },
        Metrics { name: "cores".to_owned(), value: cores as f64, attributes: Default::default() },
        Metrics { name: "uptime".to_owned(), value: uptime as f64, attributes: Default::default() },
    ])
}

#[derive(PartialEq, Debug)]
enum TelemetryToken {
    Tag(String),
    Number(f64),
    OpenBracket,
    CloseBracket,
    Quote,
    Equal,
    Comma,
}

pub struct TelemetryParser {
    line: String,
    position: usize,
    tokens: Vec<TelemetryToken>,
}

impl TelemetryParser {
    pub fn new(input: &str) -> Self {
        Self { line: input.to_string(), position: 0, tokens: Vec::new() }
    }

    pub fn parse(&mut self) -> Option<Metrics> {
        if let (Some(name), attributes, Some(value)) =
            (self.name(), self.attributes(), self.value())
        {
            Some(Metrics { name, attributes: attributes.unwrap_or_default(), value })
        } else {
            None
        }
    }

    fn name(&mut self) -> Option<String> {
        self.expecting_string()
    }

    fn attributes(&mut self) -> Option<Vec<MetricsAttribute>> {
        let mut attributes = Vec::new();

        self.expecting_special_token(TelemetryToken::OpenBracket)?;

        while let Some(attribute) = self.attribute() {
            attributes.push(attribute);
        }

        self.expecting_special_token(TelemetryToken::CloseBracket)?;
        Some(attributes)
    }

    fn attribute(&mut self) -> Option<MetricsAttribute> {
        // Eating probable comma
        _ = self.expecting_special_token(TelemetryToken::Comma);

        if let Some(attr_name) = self.expecting_string() {
            self.expecting_special_token(TelemetryToken::Equal)?;
            self.expecting_special_token(TelemetryToken::Quote)?;

            if let Some(attr_value) = self.expecting_string() {
                self.expecting_special_token(TelemetryToken::Quote)?;
                return Some(MetricsAttribute { name: attr_name, value: attr_value });
            }
        }
        None
    }

    fn expecting_special_token(&mut self, token: TelemetryToken) -> Option<TelemetryToken> {
        match self.get_token() {
            Some(tok) => {
                if tok == token {
                    Some(tok)
                } else {
                    self.put_token(tok);
                    None
                }
            }
            None => None,
        }
    }

    fn expecting_string(&mut self) -> Option<String> {
        match self.get_token() {
            Some(TelemetryToken::Tag(val)) => Some(val),
            Some(tok) => {
                self.put_token(tok);
                None
            }
            None => None,
        }
    }

    fn value(&mut self) -> Option<f64> {
        match self.get_token() {
            Some(TelemetryToken::Number(num)) => Some(num),
            Some(tok) => {
                self.put_token(tok);
                None
            }
            None => None,
        }
    }

    fn put_token(&mut self, token: TelemetryToken) {
        self.tokens.push(token)
    }

    fn get_token(&mut self) -> Option<TelemetryToken> {
        self.eat_whitespaces();

        if self.tokens.is_empty() {
            let mut string_val = String::new();

            while let Some(c) = self.line.chars().nth(self.position) {
                if let Some(token) = TelemetryParser::special_token(c) {
                    if string_val.is_empty() {
                        self.position += 1;
                        return Some(token);
                    } else {
                        break;
                    }
                } else if c.is_whitespace() {
                    break;
                } else {
                    string_val.push(c);
                    self.position += 1;
                }
            }

            if !string_val.is_empty() {
                // It might be a number, so we need to try to parse it to one
                if let Ok(float_num) = string_val.parse::<f64>() {
                    Some(TelemetryToken::Number(float_num))
                } else if let Ok(int_num) = string_val.parse::<i64>() {
                    Some(TelemetryToken::Number(int_num as f64))
                } else {
                    Some(TelemetryToken::Tag(string_val))
                }
            } else {
                None
            }
        } else {
            self.tokens.pop()
        }
    }

    fn special_token(c: char) -> Option<TelemetryToken> {
        match c {
            '=' => Some(TelemetryToken::Equal),
            '"' => Some(TelemetryToken::Quote),
            ',' => Some(TelemetryToken::Comma),
            '{' => Some(TelemetryToken::OpenBracket),
            '}' => Some(TelemetryToken::CloseBracket),
            _ => None,
        }
    }

    fn eat_whitespaces(&mut self) {
        while let Some(c) = self.line.chars().nth(self.position) {
            if c.is_whitespace() {
                self.position += 1;
            } else {
                break;
            }
        }
        self.eat_comment();
    }

    fn eat_comment(&mut self) {
        if let Some(c) = self.line.chars().nth(self.position) {
            if c == '#' {
                self.position = self.line.len();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::TelemetryParser;

    #[test]
    fn test_empty_line() {
        let metrics = TelemetryParser::new("  ").parse();
        assert_eq!(metrics, None)
    }

    #[test]
    fn test_comment_line() {
        let metrics = TelemetryParser::new("  # Some commented line we don't care about").parse();
        assert_eq!(metrics, None)
    }

    #[test]
    fn test_simple_entry() {
        let metrics = TelemetryParser::new("metric_name 12").parse();
        if let Some(metrics) = metrics {
            assert_eq!(metrics.name, "metric_name");
            assert_eq!(metrics.value, 12f64);
        } else {
            panic!("Parsed entry returned None");
        }
    }

    #[test]
    fn test_float_entry() {
        let metrics = TelemetryParser::new("metric_name 12.123").parse();
        if let Some(metrics) = metrics {
            assert_eq!(metrics.name, "metric_name");
            assert_eq!(metrics.value, 12.123f64);
        } else {
            panic!("Parsed entry returned None");
        }
    }

    #[test]
    fn test_exp_entry() {
        let metrics = TelemetryParser::new("metric_name 1.1447e+06").parse();
        if let Some(metrics) = metrics {
            assert_eq!(metrics.name, "metric_name");
            assert_eq!(metrics.value, 1144700.0f64);
        } else {
            panic!("Parsed entry returned None");
        }
    }

    #[test]
    fn test_attributed_entry() {
        let metrics = TelemetryParser::new(
            r#"metric_name{attr1_name="attr1_value",attr2_name="attr2_value"} 12"#,
        )
        .parse();
        if let Some(metrics) = metrics {
            assert_eq!(metrics.name, "metric_name");
            assert_eq!(metrics.value, 12f64);
            assert_eq!(metrics.attributes.len(), 2);
            assert_eq!(metrics.attributes[0].name, "attr1_name");
            assert_eq!(metrics.attributes[0].value, "attr1_value");
            assert_eq!(metrics.attributes[1].name, "attr2_name");
            assert_eq!(metrics.attributes[1].value, "attr2_value");
        } else {
            panic!("Parsed entry returned None");
        }
    }
}
