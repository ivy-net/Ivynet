use std::sync::Arc;

use ivynet_core::{
    avs::{names::AvsName, AvsProvider, AvsVariant},
    config::get_detailed_system_information,
    docker::dockercmd,
    error::IvyError,
    grpc::{
        backend::backend_client::BackendClient,
        messages::{Metrics, MetricsAttribute, SignedMetrics},
        tonic::{transport::Channel, Request},
    },
    signature::sign,
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
    mut client: BackendClient<Channel>,
    identity_wallet: IvyWallet,
) -> Result<(), IvyError> {
    let mut current_avs = avs_name(&avs_provider.read().await.avs);
    let mut metrics_url = None;

    loop {
        let metrics = {
            let provider = avs_provider.read().await;
            let name = avs_name(&provider.avs);
            let running = if let Some(avs) = &provider.avs { avs.is_running() } else { false };
            if running {
                match name {
                    Some(ref avs_name) => {
                        if name != current_avs {
                            metrics_url = metrics_endpoint(&avs_name).await;
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
            collect(&current_avs, &metrics_url).await
        };
        if let Ok(metrics) = metrics {
            info!("Sending metrics...");
            _ = send(&metrics, &identity_wallet, &mut client).await;
        }

        sleep(Duration::from_secs(TELEMETRY_INTERVAL_IN_MINUTES * 60)).await;
    }
}

fn avs_name(avs: &Option<Box<dyn AvsVariant>>) -> Option<String> {
    match avs {
        None => None,
        Some(avs_type) => Some(avs_type.name().to_owned()),
    }
}

async fn metrics_endpoint(avs_name: &str) -> Option<String> {
    if AvsName::EigenDA == AvsName::from(avs_name) {
        let info = dockercmd::inspect(EIGENDA_DOCKER_IMAGE_NAME).await;
        if let Some(info) = info {
            for (_, v) in info.network_settings {
                if v.port != 32005
                // TODO: Is this enough to understand what port we are looking for?
                {
                    return Some(format!("http://localhost:{}", v.port));
                }
            }
        }
    }
    None
}

async fn collect(
    avs: &Option<String>,
    metrics_url: &Option<String>,
) -> Result<Vec<Metrics>, IvyError> {
    info!("Collecting metrics for {metrics_url:?}...");
    let mut metrics = if let Some(address) = metrics_url {
        if let Ok(resp) = reqwest::get(address).await {
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
        value: if metrics_url.is_some() { 1.0 } else { 0.0 },
        attributes: if let Some(avs) = avs {
            vec![MetricsAttribute { name: "avs".to_owned(), value: avs.to_owned() }]
        } else {
            Default::default()
        },
    });

    Ok(metrics)
}

async fn send(
    metrics: &[Metrics],
    wallet: &IvyWallet,
    client: &mut BackendClient<Channel>,
) -> Result<(), IvyError> {
    let signature = sign(metrics, wallet).await?;

    client
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
