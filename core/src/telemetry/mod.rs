use std::fmt::Display;

use crate::{
    avs::names::AvsName,
    config::get_detailed_system_information,
    error::IvyError,
    ethers::types::Address,
    grpc::{
        backend::backend_client::BackendClient,
        messages::{Metrics, MetricsAttribute, NodeData, SignedMetrics, SignedNodeData},
        tonic::{transport::Channel, Request},
    },
    signature::{sign_metrics, sign_node_data},
    wallet::IvyWallet,
};
use dispatch::TelemetryDispatchHandle;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use tracing::error;
use uuid::Uuid;

pub mod dispatch;

const TELEMETRY_INTERVAL_IN_MINUTES: u64 = 1;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AvsType {
    EigenDA,
    Unknown,
}

impl Display for AvsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EigenDA => write!(f, "eigenda"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl From<&str> for AvsType {
    fn from(value: &str) -> Self {
        match value {
            "da-node" => Self::EigenDA,
            _ => Self::Unknown,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConfiguredAvs {
    pub name: String,
    pub avs_type: AvsType,
    pub metric_port: u16,
}

pub async fn listen(
    backend_client: BackendClient<Channel>,
    machine_id: Uuid,
    identity_wallet: IvyWallet,
    avses: &[ConfiguredAvs],
) -> Result<(), IvyError> {
    let dispatch = TelemetryDispatchHandle::from_client(backend_client).await;
    let mut error_rx = dispatch.error_rx.resubscribe();

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
        Ok(error) = error_rx.recv() => {
            error!("Received telemetry error: {}", error);
            Err(IvyError::CustomError(error.to_string()))
        }
    }
}

pub async fn listen_metrics(
    machine_id: Uuid,
    identity_wallet: &IvyWallet,
    avses: &[ConfiguredAvs],
    dispatch: &TelemetryDispatchHandle,
) -> Result<(), IvyError> {
    loop {
        for avs in avses {
            let node_data = NodeData {
                avs_name: avs.name.clone(),
                avs_type: avs.avs_type.to_string(),
                machine_id: machine_id.into(),
                operator_id: identity_wallet.address().as_bytes().to_vec(),
                active_set: None,
                avs_version: None,
            };

            let metrics = if let Ok(mut metrics) = fetch_telemetry_from(avs.metric_port).await {
                metrics.push(Metrics {
                    name: "running".to_owned(),
                    value: 1.0,
                    attributes: vec![MetricsAttribute {
                        name: "avs".to_owned(),
                        value: avs.name.clone(),
                    }],
                });
                metrics
            } else {
                vec![Metrics {
                    name: "running".to_owned(),
                    value: 0.0,
                    attributes: vec![MetricsAttribute {
                        name: "avs".to_owned(),
                        value: avs.name.clone(),
                    }],
                }]
            };
            let metrics_signature = sign_metrics(&metrics, identity_wallet)?;
            let signed_metrics = SignedMetrics {
                machine_id: machine_id.into(),
                avs_name: Some(avs.name.clone()),
                signature: metrics_signature.to_vec(),
                metrics: metrics.to_vec(),
            };

            let node_data_signature = sign_node_data(&node_data, identity_wallet)?;
            let signed_node_data = SignedNodeData {
                signature: node_data_signature.to_vec(),
                node_data: Some(node_data),
            };

            dispatch.send_metrics(signed_metrics).await?;
            dispatch.send_node_data(signed_node_data).await?;
        }
        // Last but not least - send system metrics
        if let Ok(system_metrics) = fetch_system_telemetry().await {
            let metrics_signature = sign_metrics(&system_metrics, identity_wallet)?;
            let signed_metrics = SignedMetrics {
                machine_id: machine_id.into(),
                avs_name: None,
                signature: metrics_signature.to_vec(),
                metrics: system_metrics.to_vec(),
            };
            dispatch.send_metrics(signed_metrics).await?;
        }

        sleep(Duration::from_secs(TELEMETRY_INTERVAL_IN_MINUTES * 60)).await;
    }
}

pub async fn delete_node_data_payload(
    identity_wallet: &IvyWallet,
    machine_id: Uuid,
    backend_client: &mut BackendClient<Channel>,
    operator_id: Address,
    avs_type: AvsName,
    avs_name: &str,
) -> Result<(), IvyError> {
    let data = NodeData {
        avs_name: avs_name.to_string(),
        avs_type: avs_type.to_string(),
        machine_id: machine_id.into(),
        operator_id: operator_id.as_bytes().to_vec(),
        active_set: None,
        avs_version: None,
    };
    let signature = sign_node_data(&data, identity_wallet)?;

    backend_client
        .delete_node_data(Request::new(SignedNodeData {
            signature: signature.to_vec(),
            node_data: Some(data),
        }))
        .await?;
    Ok(())
}

pub async fn fetch_telemetry_from(port: u16) -> Result<Vec<Metrics>, IvyError> {
    if let Ok(resp) = reqwest::get(format!("http://localhost:{}/metrics", port)).await {
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
    let (cpu_usage, ram_usage, free_ram, disk_usage, free_disk, uptime) =
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
        Metrics { name: "uptime".to_owned(), value: uptime as f64, attributes: Default::default() },
    ])
}

// async fn send(
//     metrics: &[Metrics],
//     node_data: &NodeData,
//     machine_id: Uuid,
//     avs_name: &str,
//     identity_wallet: &IvyWallet,
//     backend_client: &mut BackendClient<Channel>,
// ) -> Result<(), IvyError> {
//     let metrics_signature = sign_metrics(metrics, identity_wallet)?;
//
//     let node_data_signature = sign_node_data(node_data, identity_wallet)?;
//     backend_client
//         .metrics(Request::new(SignedMetrics {
//             machine_id: machine_id.into(),
//             avs_name: avs_name.to_string(),
//             signature: metrics_signature.to_vec(),
//             metrics: metrics.to_vec(),
//         }))
//         .await?;
//     backend_client
//         .update_node_data(Request::new(SignedNodeData {
//             signature: node_data_signature.to_vec(),
//             node_data: Some(node_data.clone()),
//         }))
//         .await?;
//     Ok(())
// }

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
