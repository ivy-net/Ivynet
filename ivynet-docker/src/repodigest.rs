use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct RepoDigest {
    pub image: String,
    pub digest: Option<String>,
}

impl RepoDigest {
    pub fn new(image: String, digest: Option<String>) -> Self {
        Self { image, digest }
    }
}

impl FromStr for RepoDigest {
    type Err = String;

    /// Parse a string in the format `image@digest` into a `RepoDigest`, for example
    /// `alpine@sha256:28ef97b8686a0b5399129e9b763d5b7e5ff03576aa5580d6f517d228e2ec1b1f`
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.rsplit_once('@');
        let (image, digest) = match parts {
            Some((image, digest)) => (image.to_string(), Some(digest.to_string())),
            None => (s.to_string(), None),
        };
        Ok(Self::new(image, digest))
    }
}

impl Display for RepoDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.digest {
            Some(digest) => write!(f, "{}@{}", self.image, digest),
            None => write!(f, "{}", self.image),
        }
    }
}
