use crate::{config::get_detailed_system_information, error::IvyError};

use ivynet_grpc::{
    backend::backend_client::BackendClient,
    messages::{Metrics, MetricsAttribute, NodeData, SignedMetrics, SignedNodeData},
    tonic::transport::Channel,
};

use dispatch::{TelemetryDispatchError, TelemetryDispatchHandle};
use ivynet_docker::dockerapi::DockerClient;
use ivynet_node_type::NodeType;
use ivynet_signer::{
    sign_utils::{sign_metrics, sign_node_data},
    IvyWallet,
};
use logs_listener::{ListenerData, LogsListenerManager};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::broadcast,
    time::{sleep, Duration},
};
use tracing::{debug, error, warn};
use uuid::Uuid;

pub mod dispatch;
pub mod logs_listener;

const TELEMETRY_INTERVAL_IN_MINUTES: u64 = 1;

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

    for node in avses {
        if let Some(container) = &docker.find_container_by_name(&node.container_name).await {
            let listener_data = ListenerData::new(
                container.clone(),
                node.clone(),
                machine_id,
                identity_wallet.clone(),
            );
            logs_listener.add_listener(listener_data).await;
        } else {
            warn!(
                "Cannot find container {}. Removing it from current listening list.",
                node.container_name
            );
        }
    }

    tokio::select! {
        metrics_err = listen_metrics(machine_id, &identity_wallet, avses, &dispatch) => {
            match metrics_err {
                Ok(_) => unreachable!("Metrics listener should never return Ok"),
                Err(err) => {
                    error!("Cannot listen for metrics ({err:?})");
                    Err(err)
                },
            }
        }
        _ = handle_telemetry_errors(error_rx) => Ok(())
    }
}

async fn handle_telemetry_errors(mut error_rx: broadcast::Receiver<TelemetryDispatchError>) {
    while let Ok(error) = error_rx.recv().await {
        error!("Received telemetry error: {}", error);
        sleep(Duration::from_secs(30)).await;
    }
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
    loop {
        let images = docker.list_images().await;
        debug!("Got images {images:#?}");
        for avs in avses {
            let mut version_hash = "".to_string();
            if let Some(inspect_data) = docker.inspect_by_container_name(&avs.container_name).await
            {
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

            // Send node data

            let node_data = NodeData {
                name: avs.assigned_name.to_owned(),
                node_type: avs.avs_type.to_string(),
                manifest: version_hash,
                metrics_alive: !metrics.is_empty(),
            };

            let node_data_signature = sign_node_data(&node_data, identity_wallet)?;
            let signed_node_data = SignedNodeData {
                machine_id: machine_id.into(),
                signature: node_data_signature.to_vec(),
                node_data: Some(node_data),
            };

            dispatch.send_node_data(signed_node_data).await?;
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

        sleep(Duration::from_secs(TELEMETRY_INTERVAL_IN_MINUTES * 60)).await;
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
        get_detailed_system_information()?;

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
