use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct RepoDigest {
    pub image: String,
    pub digest: Option<Digest>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Digest {
    /// sha256 hash of the image
    pub hash: [u8; 32],
}

impl RepoDigest {
    pub fn new(image: String, digest: Option<Digest>) -> Self {
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
            Some((image, digest)) => (image.to_string(), Some(digest.parse()?)),
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

impl FromStr for Digest {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut hash = [0; 32];
        let bytes = hex::decode(s).unwrap();
        hash.copy_from_slice(&bytes);
        Ok(Self { hash })
    }
}

impl Display for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sha256:{}", hex::encode(self.hash))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct RepoTag {
    pub image: String,
    pub tag: Option<String>,
}

impl RepoTag {
    /// Get the repository+image component of the image, for example
    /// `ghcr.io/layr-labs/eigenda/opr-node` from `ghcr.io/layr-labs/eigenda/opr-node:latest`
    pub fn repository(&self) -> String {
        self.image.clone()
    }
}

impl FromStr for RepoTag {
    type Err = String;

    /// Parse a string in the format `image:tag` into a `RepoTag`, for example
    /// `alpine:latest` If no tag is provided or string is not a valid split, tag will be `None`
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.rsplit_once(':');
        let (image, tag) = match parts {
            Some((image, tag)) => (image.to_string(), Some(tag.to_string())),
            None => (s.to_string(), None),
        };
        Ok(Self { image, tag })
    }
}

impl Display for RepoTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.tag {
            Some(tag) => write!(f, "{}:{}", self.image, tag),
            None => write!(f, "{}", self.image),
        }
    }
}
