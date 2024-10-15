use std::sync::Arc;

use ivynet_core::{
    avs::{names::AvsName, AvsProvider, AvsVariant},
    config::get_detailed_system_information,
    docker::dockercmd,
    error::IvyError,
    ethers::types::Address,
    grpc::{
        backend::backend_client::BackendClient,
        messages::{
            Metrics, MetricsAttribute, NodeData, SignedDeleteNodeData, SignedMetrics,
            SignedNodeData,
        },
        tonic::{transport::Channel, Request},
    },
    signature::{sign_delete_node_data, sign_metrics, sign_node_data},
    wallet::IvyWallet,
};
use tokio::{
    sync::RwLock,
    time::{sleep, Duration},
};
use tracing::info;

const EIGENDA_DOCKER_IMAGE_NAME: &str = "eigenda-native-node";

const TELEMETRY_INTERVAL_IN_MINUTES: u64 = 1;

pub async fn listen(
    avs_provider: Arc<RwLock<AvsProvider>>,
    mut backend_client: BackendClient<Channel>,
    identity_wallet: IvyWallet,
) -> Result<(), IvyError> {
    let mut current_avs = avs_name(&avs_provider.read().await.avs);
    let mut metrics_url = None;

    loop {
        let (metrics, node_data) = {
            let provider = avs_provider.read().await;
            let name = avs_name(&provider.avs);
            let running = if let Some(avs) = &provider.avs { avs.is_running() } else { false };
            println!("Is running {running:?} the ava {name:?}");

            if running {
                match name {
                    Some(ref avs_name) => {
                        if name != current_avs {
                            metrics_url = metrics_endpoint(&avs_name).await;
                            println!("Metrics url is {metrics_url:?}");
                            current_avs = name;
                        }
                    }
                    None => {
                        metrics_url = None;
                    }
                }
            } else {
                metrics_url = None;
            }
            collect(&avs_provider, &metrics_url).await?
        };
        info!("Sending metrics...");
        _ = send_metrics(&metrics, &identity_wallet, &mut backend_client).await;
        _ = send_node_data_payload(&identity_wallet, &mut backend_client, &node_data).await;
        sleep(Duration::from_secs(TELEMETRY_INTERVAL_IN_MINUTES * 60)).await;
    }
}

fn avs_name(avs: &Option<Box<dyn AvsVariant>>) -> Option<String> {
    match avs {
        None => None,
        Some(avs_type) => Some(avs_type.name().to_owned().to_string()),
    }
}

async fn metrics_endpoint(avs_name: &str) -> Option<String> {
    if AvsName::EigenDA == AvsName::from(avs_name) {
        let info = dockercmd::inspect(EIGENDA_DOCKER_IMAGE_NAME).await;
        if let Some(info) = info {
            for (_, v) in info.network_settings.ports {
                for ep in v {
                    if let Ok(port) = ep.port.parse::<u16>() {
                        let url = format!("http://localhost:{}/metrics", port);
                        if reqwest::get(&url).await.is_ok() {
                            return Some(url);
                        }
                    }
                }
            }
        }
    }
    None
}

async fn collect(
    avs_provider: &Arc<RwLock<AvsProvider>>,
    metrics_url: &Option<String>,
) -> Result<(Vec<Metrics>, NodeData), IvyError> {
    let provider = avs_provider.read().await;
    let avs = &provider.avs;
    // Depending on currently running avs, we decide how to fetch
    let (avs_name, metrics_location, address, running) = match avs {
        None => (None, None, None, false),
        Some(avs_type) => {
            match avs_type.name() {
                AvsName::EigenDA => (
                    Some(AvsName::EigenDA.to_string()),
                    Some("http://localhost:9092/metrics"),
                    Some(format!("{:?}", provider.provider.address())),
                    avs_type.is_running(),
                ),
                _ => (Some(avs_type.name().to_string()), None, None, avs_type.is_running()), // * that one */
            }
        }
    };

    let address = format!("{:?}", provider.provider.address());
    let running = if let Some(avs) = avs { avs.is_running() } else { false };
    let avs_name: Option<AvsName> = if let Some(avs) = avs { Some(avs.name()) } else { None };

    info!("Collecting metrics for {metrics_url:?}...");
    let mut metrics = if let Some(metrics_url) = metrics_url {
        if let Ok(resp) = reqwest::get(metrics_url).await {
            if let Ok(body) = resp.text().await {
                let metrics = body
                    .split('\n')
                    .filter_map(|line| TelemetryParser::new(line).parse())
                    .collect::<Vec<_>>();

                metrics
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let node_data = if let Some(avs) = avs {
        NodeData {
            operator_id: provider.provider.address().as_bytes().to_vec(),
            avs_name: match avs_name.clone() {
                Some(avs_name) => avs_name.to_string(),
                None => "".to_string(),
            },
            avs_version: {
                if let Ok(version) = avs.version() {
                    version.to_string()
                } else {
                    "0.0.0".to_string()
                }
            },
            active_set: avs.active_set(provider.provider.clone()).await,
        }
    } else {
        NodeData {
            operator_id: provider.provider.address().as_bytes().to_vec(),
            avs_name: "".to_string(),
            avs_version: "0.0.0".to_string(),
            active_set: false,
        }
    };

    // Now we need to add basic metrics
    let (cpu_usage, ram_usage, disk_usage, free_space, uptime) = get_detailed_system_information()?;

    metrics.push(Metrics {
        name: "cpu_usage".to_owned(),
        value: cpu_usage,
        attributes: Default::default(),
    });

    metrics.push(Metrics {
        name: "ram_usage".to_owned(),
        value: ram_usage as f64,
        attributes: Default::default(),
    });

    metrics.push(Metrics {
        name: "disk_usage".to_owned(),
        value: disk_usage as f64,
        attributes: Default::default(),
    });

    metrics.push(Metrics {
        name: "free_space".to_owned(),
        value: free_space as f64,
        attributes: Default::default(),
    });

    metrics.push(Metrics {
        name: "uptime".to_owned(),
        value: uptime as f64,
        attributes: Default::default(),
    });

    metrics.push(Metrics {
        name: "running".to_owned(),
        value: if running { 1.0 } else { 0.0 },
        attributes: if let Some(avs_name) = avs_name {
            vec![
                MetricsAttribute { name: "avs".to_owned(), value: avs_name.to_owned().to_string() },
                MetricsAttribute {
                    name: "chain".to_owned(),
                    value: {
                        match provider.chain().await {
                            Ok(chain) => chain.to_string(),
                            Err(_) => "unknown".to_string(),
                        }
                    },
                },
                MetricsAttribute { name: "operator_id".to_owned(), value: address },
                MetricsAttribute {
                    name: "active_set".to_owned(),
                    value: node_data.active_set.to_string(),
                },
                MetricsAttribute {
                    name: "version".to_owned(),
                    value: node_data.avs_version.to_string(),
                },
            ]
        } else {
            Default::default()
        },
    });

    Ok((metrics, node_data))
}

async fn send_metrics(
    metrics: &[Metrics],
    identity_wallet: &IvyWallet,
    backend_client: &mut BackendClient<Channel>,
) -> Result<(), IvyError> {
    let signature = sign_metrics(metrics, identity_wallet)?;

    backend_client
        .metrics(Request::new(SignedMetrics {
            signature: signature.to_vec(),
            metrics: metrics.to_vec(),
        }))
        .await?;
    Ok(())
}

pub async fn send_node_data_payload(
    identity_wallet: &IvyWallet,
    backend_client: &mut BackendClient<Channel>,
    node_data: &NodeData,
) -> Result<(), IvyError> {
    let signature = sign_node_data(node_data, identity_wallet)?;

    let signed_node_data =
        SignedNodeData { signature: signature.to_vec(), node_data: Some(node_data.clone()) };

    let request = Request::new(signed_node_data);
    backend_client.node_data(request).await?;
    Ok(())
}

pub async fn delete_node_data_payload(
    identity_wallet: &IvyWallet,
    backend_client: &mut BackendClient<Channel>,
    operator_id: Address,
    avs_name: AvsName,
) -> Result<(), IvyError> {
    let signature = sign_delete_node_data(operator_id, avs_name.to_string(), identity_wallet)?;

    let signed_node_data = SignedDeleteNodeData {
        signature: signature.to_vec(),
        operator_id: operator_id.as_bytes().to_vec(),
        avs_name: avs_name.to_string(),
    };

    let request = Request::new(signed_node_data);
    backend_client.delete_node_data(request).await?;
    Ok(())
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

struct TelemetryParser {
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
