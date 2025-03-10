use std::str::FromStr;

pub struct ExposedPort {
    port: u16,
    protocol: String,
}

impl ExposedPort {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn protocol(&self) -> &str {
        &self.protocol
    }
}

impl FromStr for ExposedPort {
    type Err = PortError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(PortError::InvalidFormat);
        }
        let port = parts[0].parse().unwrap();
        let protocol = parts[1].to_string();
        Ok(Self { port, protocol })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PortError {
    #[error("Invalid format")]
    InvalidFormat,
}
