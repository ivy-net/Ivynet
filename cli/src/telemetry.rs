use std::sync::Arc;

use ivynet_core::{
    avs::{names::AvsName, AvsProvider},
    config::get_detailed_system_information,
    error::IvyError,
    grpc::{
        backend::backend_client::BackendClient,
        messages::{Metrics, MetricsAttribute, SignedMetrics},
        tonic::{transport::Channel, Request},
    },
    signature::sign_metrics,
    wallet::IvyWallet,
};
use tokio::{
    sync::RwLock,
    time::{sleep, Duration},
};
use tracing::info;

const TELEMETRY_INTERVAL_IN_MINUTES: u64 = 1;

pub async fn listen(
    avs_provider: Arc<RwLock<AvsProvider>>,
    mut backend_client: BackendClient<Channel>,
    identity_wallet: IvyWallet,
) -> Result<(), IvyError> {
    loop {
        let metrics = { collect(&avs_provider).await }?;
        info!("Sending metrics...");
        _ = send_metrics(&metrics, &identity_wallet, &mut backend_client).await;
        sleep(Duration::from_secs(TELEMETRY_INTERVAL_IN_MINUTES * 60)).await;
    }
}

async fn collect(avs_provider: &Arc<RwLock<AvsProvider>>) -> Result<Vec<Metrics>, IvyError> {
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

    info!("Collecting metrics for {address:?} ({running})...");
    let mut metrics = if let Some(metrics_location) = metrics_location {
        if let Ok(resp) = reqwest::get(metrics_location).await {
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
                MetricsAttribute { name: "avs".to_owned(), value: avs_name.to_owned() },
                MetricsAttribute {
                    name: "chain".to_owned(),
                    value: {
                        match provider.chain().await {
                            Ok(chain) => chain.to_string(),
                            Err(_) => "unknown".to_string(),
                        }
                    },
                },
                MetricsAttribute {
                    name: "operator_id".to_owned(),
                    value: address.unwrap_or("".to_string()).to_string(),
                },
                MetricsAttribute {
                    name: "active_set".to_owned(),
                    value: {
                        if let Some(avs) = avs {
                            avs.active_set(provider.provider.clone()).await.to_string()
                        } else {
                            "false".to_string()
                        }
                    },
                },
                MetricsAttribute {
                    name: "version".to_owned(),
                    value: {
                        if let Some(avs) = avs {
                            if let Ok(version) = avs.version() {
                                version.to_string()
                            } else {
                                "0.0.0".to_string()
                            }
                        } else {
                            "0.0.0".to_string()
                        }
                    },
                },
            ]
        } else {
            Default::default()
        },
    });

    println!("METRICS: {:#?}", metrics);

    Ok(metrics)
}

async fn send_metrics(
    metrics: &[Metrics],
    wallet: &IvyWallet,
    backend_client: &mut BackendClient<Channel>,
) -> Result<(), IvyError> {
    let signature = sign_metrics(metrics, wallet)?;

    backend_client
        .metrics(Request::new(SignedMetrics {
            signature: signature.to_vec(),
            metrics: metrics.to_vec(),
        }))
        .await?;
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
