pub mod alerts_active;
pub mod alerts_historical;

#[derive(Debug, Clone, Copy)]
pub enum AlertType {
    DEBUG = 0,
}

impl From<i64> for AlertType {
    fn from(value: i64) -> Self {
        match value {
            0 => AlertType::DEBUG,
            _ => panic!("Unknown alert type: {}", value),
        }
    }
}
