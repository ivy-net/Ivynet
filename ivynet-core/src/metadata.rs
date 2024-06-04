use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    metadata_uri: String,
    logo_uri: String,
    favicon_uri: String,
}

impl Metadata {
    #[allow(dead_code)]
    fn new(metadata_uri: &str, logo_uri: &str, favicon_uri: &str) -> Self {
        Self {
            metadata_uri: metadata_uri.to_string(),
            logo_uri: logo_uri.to_string(),
            favicon_uri: favicon_uri.to_string(),
        }
    }
}
