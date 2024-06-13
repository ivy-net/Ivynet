use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub metadata_uri: String,
    pub logo_uri: String,
    pub favicon_uri: String,
}

impl Metadata {
    pub fn new(metadata_uri: &str, logo_uri: &str, favicon_uri: &str) -> Self {
        Self {
            metadata_uri: metadata_uri.to_string(),
            logo_uri: logo_uri.to_string(),
            favicon_uri: favicon_uri.to_string(),
        }
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self::new("", "", "")
    }
}
