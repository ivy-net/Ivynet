use std::fmt;

#[derive(Debug)]
pub enum AVSError {
    NoStake,
}

impl fmt::Display for AVSError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AVSError::NoStake => write!(f, "Operator has no stake in native ETH or WETH"),
        }
    }
}

impl std::error::Error for AVSError {}
