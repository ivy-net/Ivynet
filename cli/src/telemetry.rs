use std::sync::Arc;

use ivynet_core::{
    avs::{instance::AvsType, AvsProvider},
    error::IvyError,
    grpc::messages::{Metrics, MetricsAttribute},
    rpc_management::IvyProvider,
};
use tokio::{
    sync::RwLock,
    time::{sleep, Duration},
};
use tracing::debug;

pub async fn listen(avs_provider: Arc<RwLock<AvsProvider>>) -> Result<(), IvyError> {
    loop {
        let provider = avs_provider.read().await;
        let metrics = collect(&provider.avs).await?;
        debug!("Collected metrics {metrics:?}");
        if let Some(metrics) = metrics {
            send(&metrics, &provider.provider).await?;
        }
        sleep(Duration::from_secs(5 * 60)).await;
    }
}
async fn collect(avs: &Option<AvsType>) -> Result<Option<Vec<Metrics>>, IvyError> {
    // Depending on currently running avs, we decide how to fetch
    let address = match avs {
        None => None,
        Some(avs_type) => {
            match avs_type {
                AvsType::EigenDA(_) => Some("http://localhost:9092/metrics"),
                AvsType::AltLayer(_) => None, //TODO: Still don't know how to fetch data from that one
            }
        }
    };

    if let Some(address) = address {
        let body = reqwest::get(address).await?.text().await?;

        let metrics = body
            .split('\n')
            .filter_map(|line| TelemetryParser::new(line).parse())
            .collect::<Vec<_>>();

        Ok(Some(metrics))
    } else {
        Ok(None)
    }
}

async fn send(_metrics: &[Metrics], _provider: &IvyProvider) -> Result<(), IvyError> {
    Ok(())
}

#[derive(PartialEq)]
enum TelemetryToken {
    Tag(String),
    OpenBracket,
    CloseBracket,
    Qoute,
    Equal,
    Comma,
    Number(f64),
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
            Some(Metrics {
                name,
                attributes: attributes.unwrap_or_default(),
                value,
            })
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
            self.expecting_special_token(TelemetryToken::Qoute)?;

            if let Some(attr_value) = self.expecting_string() {
                self.expecting_special_token(TelemetryToken::Qoute)?;
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
            let mut chars = self.line.chars();
            let mut string_val = String::new();

            while let Some(c) = chars.nth(self.position) {
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
            '"' => Some(TelemetryToken::Qoute),
            ',' => Some(TelemetryToken::Comma),
            '{' => Some(TelemetryToken::OpenBracket),
            '}' => Some(TelemetryToken::CloseBracket),
            _ => None,
        }
    }

    fn eat_whitespaces(&mut self) {
        let mut chars = self.line.chars();
        while let Some(c) = chars.nth(self.position) {
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
