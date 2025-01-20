use std::fmt;

use serde::{Deserialize, Serialize};
use strum::EnumIter;
use tokio::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum RegistryType {
    DockerHub,
    OtherDockerHub,
    Github,
    GoogleCloud,
    AWS,
    Chainbase,
    Othentic,
    Local,
}

impl RegistryType {
    pub fn get_registry_hosts() -> Vec<&'static str> {
        vec![
            "registry-1.docker.io",
            "docker.io",
            "ghcr.io",
            "gcr.io",
            "public.ecr.aws",
            "repository.chainbase.com",
        ]
    }

    pub fn from_host(host: &str) -> Option<Self> {
        match host {
            "registry-1.docker.io" => Some(Self::DockerHub),
            "docker.io" => Some(Self::OtherDockerHub),
            "ghcr.io" => Some(Self::Github),
            "gcr.io" => Some(Self::GoogleCloud),
            "public.ecr.aws" => Some(Self::AWS),
            "repository.chainbase.com" => Some(Self::Chainbase),
            "othentic" => Some(Self::Othentic),
            "local" => Some(Self::Local),
            _ => None,
        }
    }

    pub fn batch_size(&self) -> usize {
        match self {
            Self::AWS => 5, // AWS has stricter rate limits
            _ => 10,
        }
    }

    pub fn retry_delay(&self) -> Duration {
        match self {
            Self::AWS => Duration::from_secs(5),
            _ => Duration::from_secs(1),
        }
    }

    pub fn max_retries(&self) -> u32 {
        match self {
            Self::AWS => 12,
            _ => 4,
        }
    }
}

impl fmt::Display for RegistryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let registry = match self {
            Self::OtherDockerHub => "docker.io",
            Self::DockerHub => "registry-1.docker.io",
            Self::Github => "ghcr.io",
            Self::GoogleCloud => "gcr.io",
            Self::AWS => "public.ecr.aws",
            Self::Chainbase => "repository.chainbase.com",
            Self::Othentic => "othentic",
            Self::Local => "local",
        };
        write!(f, "{}", registry)
    }
}
