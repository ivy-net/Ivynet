use std::sync::Arc;

use ivynet_core::{
    avs::{AvsProvider, AvsVariant},
    config::get_system_information,
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

const TELEMETRY_INTERVAL_IN_MINUTES: u64 = 1;

pub async fn listen(
    avs_provider: Arc<RwLock<AvsProvider>>,
    mut client: BackendClient<Channel>,
    identity_wallet: IvyWallet,
) -> Result<(), IvyError> {
    loop {
        let metrics = {
            let provider = avs_provider.read().await;
            collect(&provider.avs).await
        };
        if let Ok(metrics) = metrics {
            _ = send(&metrics, &identity_wallet, &mut client).await;
        }

        sleep(Duration::from_secs(TELEMETRY_INTERVAL_IN_MINUTES * 60)).await;
    }
}

async fn collect(avs: &Option<Box<dyn AvsVariant>>) -> Result<Vec<Metrics>, IvyError> {
    // Depending on currently running avs, we decide how to fetch
    let (avs, address, running) = match avs {
        None => (None, None, false),
        Some(avs_type) => {
            match avs_type.name() {
                "eigenda" => {
                    (Some("eigenda"), Some("http://localhost:9092/metrics"), avs_type.running())
                }
                _ => (Some(avs_type.name()), None, avs_type.running()), // * that one */
            }
        }
    };

    let mut metrics = if let Some(address) = address {
        let body = reqwest::get(address).await?.text().await?;

        let metrics = body
            .split('\n')
            .filter_map(|line| TelemetryParser::new(line).parse())
            .collect::<Vec<_>>();

        metrics
    } else {
        Vec::new()
    };

    // Now we need to add basic metrics
    let (cpus, ram, free_space) = get_system_information()?;

    metrics.push(Metrics {
        name: "cpus".to_owned(),
        value: cpus as f64,
        attributes: Default::default(),
    });

    metrics.push(Metrics {
        name: "ram".to_owned(),
        value: ram as f64,
        attributes: Default::default(),
    });

    metrics.push(Metrics {
        name: "free_space".to_owned(),
        value: free_space as f64,
        attributes: Default::default(),
    });

    metrics.push(Metrics {
        name: "running".to_owned(),
        value: if running { 1.0 } else { 0.0 },
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
