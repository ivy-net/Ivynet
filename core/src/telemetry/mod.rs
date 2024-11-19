use crate::{
    config::get_detailed_system_information,
    docker::dockerapi::{Container, DockerClient},
    error::IvyError,
    ethers::types::{Address, Chain},
    grpc::{
        backend::backend_client::BackendClient,
        messages::{Metrics, MetricsAttribute, NodeData, SignedMetrics, SignedNodeData},
        tonic::{transport::Channel, Request},
    },
    node_type::NodeType,
    signature::{sign_metrics, sign_node_data},
    wallet::IvyWallet,
};
use dispatch::TelemetryDispatchHandle;
use tokio::time::{sleep, Duration};
use tracing::{error, info};
use uuid::Uuid;

pub mod dispatch;

const TELEMETRY_INTERVAL_IN_MINUTES: u64 = 1;

pub async fn listen(
    backend_client: BackendClient<Channel>,
    machine_id: Uuid,
    identity_wallet: IvyWallet,
) -> Result<(), IvyError> {
    let machine_id = machine_id.to_string();
    let docker = DockerClient::default();
    let dispatch = TelemetryDispatchHandle::from_client(backend_client).await;
    let mut error_rx = dispatch.error_rx.resubscribe();

    tokio::select! {
        metrics_err = listen_metrics(&docker, &machine_id, &identity_wallet, &dispatch) => {
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

    //     loop {
    //         let (metrics, node_data) = {
    //             match name {
    //                 Some(ref avs_name) => {
    //                     //TODO: Forcing update of metrics_url every time
    //                     // because caching it makes the endpoint stay as "None"
    //                     // even after the endpoint has been built
    //                     // More elegant solution needed in the future
    //                     metrics_url = metrics_endpoint(avs_name).await;
    //                 }
    //             }
    //             let node_data = node_data(&provider.avs, &name, machine_id,
    // &provider.provider).await?;             (collect(&name, &metrics_url, &node_data,
    // provider.chain().await.ok()).await, node_data)         };
    //         if let Ok(metrics) = metrics {
    //             info!("Sending metrics...");
    //             // TODO: This avs_name has to be the name of the container. But the observability
    // is             // not ready for it just yet
    //             match send(&metrics, &node_data, machine_id, "", &identity_wallet, &mut
    // backend_client)                 .await
    //             {
    //                 Ok(_) => {}
    //                 Err(err) => error!("Cannot send metrics to backend ({err:?})"),
    //             }
    //         }
    //
    //         sleep(Duration::from_secs(TELEMETRY_INTERVAL_IN_MINUTES * 60)).await;
    //     }
}

pub async fn listen_metrics(
    docker: &DockerClient,
    machine_id: &str,
    identity_wallet: &IvyWallet,
    dispatch: &TelemetryDispatchHandle,
) -> Result<(), IvyError> {
    loop {
        let node_containers = docker.find_all_node_containers().await;
        let mut nodes_with_metrics: Vec<(&Container, String)> = Vec::new();

        for node in node_containers.iter() {
            let ports = node.public_ports();

            for port in ports {
                let url = format!("http://localhost:{}/metrics", port);
                if reqwest::get(&url).await.is_ok() {
                    nodes_with_metrics.push((node, url));
                }
            }
        }

        for node in nodes_with_metrics {
            let (container, url) = node;
            let avs_name = container.image().ok_or(IvyError::DockerImageError)?.to_string();
            let node_data = NodeData {
                avs_name: avs_name.clone(),
                avs_type: NodeType::try_from_docker_image_name(&avs_name)?.to_string(),
                machine_id: machine_id.into(),
                operator_id: identity_wallet.address().as_bytes().to_vec(),
                active_set: None,
                avs_version: None,
            };

            // TODO: node_data does not accept an option, but collect does. Is that correct?
            let metrics = collect(&Some(avs_name.clone()), &Some(url), &node_data, None).await;

            if let Ok(metrics) = metrics {
                info!("Sending metrics...");

                let metrics_signature = sign_metrics(&metrics, identity_wallet)?;
                let signed_metrics = SignedMetrics {
                    machine_id: machine_id.into(),
                    avs_name,
                    signature: metrics_signature.to_vec(),
                    metrics: metrics.to_vec(),
                };

                let node_data_signature = sign_node_data(&node_data, identity_wallet)?;
                let signed_node_data = SignedNodeData {
                    signature: node_data_signature.to_vec(),
                    node_data: Some(node_data),
                };

                // client.metrics(Request::new(signed_metrics)).await;
                // client.update_node_data(Request::new(signed_node_data)).await;

                dispatch.send_metrics(signed_metrics).await?;
                dispatch.send_node_data(signed_node_data).await?;
            } else {
                error!("Cannot collect metrics for container {:?}", container.image());
            }
        }
        sleep(Duration::from_secs(TELEMETRY_INTERVAL_IN_MINUTES * 60)).await;
    }
}

pub async fn delete_node_data_payload(
    identity_wallet: &IvyWallet,
    machine_id: Uuid,
    backend_client: &mut BackendClient<Channel>,
    operator_id: Address,
    avs_type: NodeType,
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

// async fn metrics_endpoint(docker_image_name: &str) -> Option<String> {
//     let node_type = NodeType::try_from_docker_image_name(docker_image_name)?;
//     let url: Option<String> = match node_type {
//         NodeType::EigenDA => {
//             let container_info = dockerapi::inspect(node_type.default_docker_image_name()).await;
//         }
//         _ => None,
//     };
//     if let Ok(AvsName::EigenDA) = AvsName::try_from(avs_name) {
//         let info = dockerapi::inspect(EIGENDA_DOCKER_IMAGE_NAME).await;
//         if let Some(info) = info {
//             let ports = dockerapi::get_active_ports(&info);
//             debug!("Ports: {:?}", ports);
//             for port in ports {
//                 let url = format!("http://localhost:{}/metrics", port);
//                 if reqwest::get(&url).await.is_ok() {
//                     return Some(url);
//                 }
//             }
//         }
//     }
//     todo!()
// }

// TODO: Ask about if/else for this function
// async fn node_data(
//     avs_name: &Option<String>,
//     machine_id: Uuid,
//     provider: &Arc<IvyProvider>,
// ) -> Result<NodeData, IvyError> {
//     Ok(if let Some(avs) = avs {
//         NodeData {
//             avs_type: avs.name().to_string(),
//             machine_id: machine_id.into(),
//             operator_id: provider.address().as_bytes().to_vec(),
//             avs_name: avs_name.clone().unwrap_or("".to_string()),
//             avs_version: avs.version().ok().map(|v| v.to_string()),
//             active_set: Some(avs.active_set(provider.clone()).await),
//         }
//     } else {
//         NodeData {
//             avs_type: "".to_string(),
//             machine_id: machine_id.into(),
//             operator_id: provider.address().as_bytes().to_vec(),
//             avs_name: "".to_string(),
//             avs_version: Some("0.0.0".to_string()),
//             active_set: Some(false),
//         }
//     })
// }

async fn collect(
    avs: &Option<String>,
    metrics_url: &Option<String>,
    node_data: &NodeData,
    chain: Option<Chain>,
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
    let (cores, cpu_usage, ram_usage, free_ram, disk_usage, free_disk, uptime) =
        get_detailed_system_information()?;

    metrics.push(Metrics {
        name: "cores".to_owned(),
        value: cores as f64,
        attributes: Default::default(),
    });

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
        name: "free_ram".to_owned(),
        value: free_ram as f64,
        attributes: Default::default(),
    });
    metrics.push(Metrics {
        name: "disk_usage".to_owned(),
        value: disk_usage as f64,
        attributes: Default::default(),
    });

    metrics.push(Metrics {
        name: "free_disk".to_owned(),
        value: free_disk as f64,
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
            vec![
                MetricsAttribute { name: "avs".to_owned(), value: avs.to_owned() },
                MetricsAttribute {
                    name: "chain".to_owned(),
                    value: {
                        match chain {
                            Some(chain) => chain.to_string(),
                            None => "unknown".to_string(),
                        }
                    },
                },
                MetricsAttribute {
                    name: "operator_id".to_owned(),
                    value: format!("{:?}", Address::from_slice(&node_data.operator_id)),
                },
                MetricsAttribute {
                    name: "active_set".to_owned(),
                    value: node_data.active_set.unwrap_or(false).to_string(),
                },
                MetricsAttribute {
                    name: "version".to_owned(),
                    value: node_data
                        .avs_version
                        .as_ref()
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "0.0.0".to_string()),
                },
            ]
        } else {
            Default::default()
        },
    });

    Ok(metrics)
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
