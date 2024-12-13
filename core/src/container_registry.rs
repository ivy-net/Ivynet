use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ContainerRegistry {
    DockerHub,
    Github,
    GoogleCloud,
    AWS,
    Chainbase,
}

impl ContainerRegistry {
    pub fn from_host(host: &str) -> Option<Self> {
        match host {
            "registry-1.docker.io" | "docker.io" => Some(Self::DockerHub),
            "ghcr.io" => Some(Self::Github),
            "gcr.io" => Some(Self::GoogleCloud),
            "public.ecr.aws" => Some(Self::AWS),
            "repository.chainbase.com" => Some(Self::Chainbase),
            _ => None,
        }
    }
}

impl fmt::Display for ContainerRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let registry = match self {
            Self::DockerHub => "registry-1.docker.io",
            Self::Github => "ghcr.io",
            Self::GoogleCloud => "gcr.io",
            Self::AWS => "public.ecr.aws",
            Self::Chainbase => "repository.chainbase.com",
        };
        write!(f, "{}", registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_from_host() {
        assert_eq!(ContainerRegistry::from_host("ghcr.io"), Some(ContainerRegistry::Github));
        assert_eq!(ContainerRegistry::from_host("invalid"), None);
    }

    #[test]
    fn test_registry_host() {
        assert_eq!(ContainerRegistry::Github.to_string(), "ghcr.io");
    }
}
