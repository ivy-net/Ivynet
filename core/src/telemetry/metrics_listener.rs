use std::time::Duration;

use ivynet_docker::dockerapi::DockerClient;
use reqwest::Client;
use tokio::{sync::mpsc, time::sleep};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{
    config::get_detailed_system_information,
    error::IvyError,
    grpc::messages::{Metrics, MetricsAttribute, NodeData, SignedMetrics, SignedNodeData},
    signature::{sign_metrics, sign_node_data},
    wallet::IvyWallet,
};

use super::{dispatch::TelemetryDispatchHandle, ConfiguredAvs};

const TELEMETRY_INTERVAL_IN_MINUTES: u64 = 1;

#[derive(Debug, Clone)]
pub struct MetricsListenerHandle {
    tx: mpsc::Sender<MetricsListenerAction>,
}

impl MetricsListenerHandle {
    pub fn new(
        machine_id: Uuid,
        identity_wallet: &IvyWallet,
        avses: &[ConfiguredAvs],
        dispatch: &TelemetryDispatchHandle,
    ) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let listener = MetricsListener::new(
            machine_id,
            identity_wallet.clone(),
            avses.to_vec(),
            dispatch.clone(),
            rx,
        );
        tokio::spawn(listener.run());
        Self { tx }
    }

    pub async fn add_node(&self, avs: ConfiguredAvs) -> Result<(), MetricsListenerError> {
        self.tx.send(MetricsListenerAction::AddNode(avs)).await?;
        Ok(())
    }

    pub async fn remove_node(&self, avs: ConfiguredAvs) -> Result<(), MetricsListenerError> {
        self.tx.send(MetricsListenerAction::RemoveNode(avs)).await?;
        Ok(())
    }

    pub async fn remove_node_by_name(
        &self,
        container_name: &str,
    ) -> Result<(), MetricsListenerError> {
        self.tx.send(MetricsListenerAction::RemoveNodeByName(container_name.to_string())).await?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MetricsListenerError {
    #[error("Failed to send metrics: {0}")]
    SendError(#[from] mpsc::error::SendError<MetricsListenerAction>),
}

/// The MetricsListener is responsible for listening to metrics from the machine and sending them
/// to the telemetry dispatch. It is also responsible for listening to changes in the AVS list and
/// updating the AVS list accordingly. `avses` would probably be better represented by a set keyed
/// to the container_name name, which is unique per docker sysem.
pub struct MetricsListener {
    machine_id: Uuid,
    identity_wallet: IvyWallet,
    avses: Vec<ConfiguredAvs>,
    dispatch: TelemetryDispatchHandle,
    rx: mpsc::Receiver<MetricsListenerAction>,
}

impl MetricsListener {
    pub fn new(
        machine_id: Uuid,
        identity_wallet: IvyWallet,
        avses: Vec<ConfiguredAvs>,
        dispatch: TelemetryDispatchHandle,
        rx: mpsc::Receiver<MetricsListenerAction>,
    ) -> Self {
        Self { machine_id, identity_wallet, avses, dispatch, rx }
    }

    pub async fn run(mut self) {
        let mut interval =
            tokio::time::interval(Duration::from_secs(60 * TELEMETRY_INTERVAL_IN_MINUTES));
        // broadcast metrics when we get an update event or once a minute, whichever comes first
        loop {
            let res = tokio::select! {
                _ = interval.tick() => {
                    self.broadcast_metrics().await
                }
                Some(action) = self.rx.recv() => {
                    self.handle_action(action).await
                }
            };
            if let Err(e) = res {
                error!("Failed to broadcast metrics: {:#?}", e);
            }
        }
    }

    async fn broadcast_metrics(&self) -> Result<(), IvyError> {
        report_metrics(
            self.machine_id,
            &self.identity_wallet,
            self.avses.as_slice(),
            &self.dispatch,
        )
        .await
    }

    async fn handle_action(&mut self, action: MetricsListenerAction) -> Result<(), IvyError> {
        match action {
            MetricsListenerAction::AddNode(avs) => {
                // if container with name already exists, replace avs_type and metric_port
                if let Some(existing) =
                    self.avses.iter_mut().find(|x| x.container_name == avs.container_name)
                {
                    existing.avs_type = avs.avs_type;
                    existing.metric_port = avs.metric_port;
                } else {
                    self.avses.push(avs);
                }
                self.broadcast_metrics().await
            }
            MetricsListenerAction::RemoveNode(avs) => {
                self.avses.retain(|x| x.container_name != avs.container_name);
                self.broadcast_metrics().await
            }
            MetricsListenerAction::RemoveNodeByName(container_name) => {
                let avs_num = self.avses.len();
                self.avses.retain(|x| x.container_name != container_name);
                if avs_num != self.avses.len() {
                    info!("Detected container stop: {}", container_name);
                }
                self.broadcast_metrics().await
            }
        }
    }
}

#[derive(Clone)]
pub enum MetricsListenerAction {
    AddNode(ConfiguredAvs),
    RemoveNode(ConfiguredAvs),
    /// Remove a node by its container name
    RemoveNodeByName(String),
}

pub async fn report_metrics(
    machine_id: Uuid,
    identity_wallet: &IvyWallet,
    avses: &[ConfiguredAvs],
    dispatch: &TelemetryDispatchHandle,
) -> Result<(), IvyError> {
    let docker = DockerClient::default();
    let images = docker.list_images().await;
    debug!("Got images {images:#?}");
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
    Ok(())
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
