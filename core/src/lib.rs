pub mod avs;
pub mod bls;
pub mod config;
pub mod constants;
pub mod directory;
pub mod download;
pub mod eigen;
pub mod env_parser;
pub mod error;
pub mod grpc;
pub mod io;
pub mod ivy_yaml;
pub mod keychain;
pub mod keys;
pub mod metadata;
pub mod signature;
pub mod system;
pub mod telemetry;
pub mod utils;
pub mod wallet;

use std::collections::HashMap;

pub use blsful::{Bls12381G1Impl, PublicKey, SecretKey};
pub use ethers;

use ethers::{
    middleware::{signer::SignerMiddlewareError, SignerMiddleware},
    providers::{Http, Provider},
};
use ivynet_docker::RegistryType;
use ivynet_node_type::NodeType;
use tracing::warn;
use wallet::IvyWallet;

pub type IvyProvider = SignerMiddleware<Provider<Http>, IvyWallet>;
pub type IvyProviderError = SignerMiddlewareError<Provider<Http>, IvyWallet>;

fn extract_image_name(image_name: &str) -> String {
    RegistryType::get_registry_hosts()
        .into_iter()
        .find_map(|registry| {
            image_name.contains(registry).then(|| {
                image_name
                    .split(&registry)
                    .last()
                    .unwrap_or(image_name)
                    .trim_start_matches('/')
                    .to_string()
            })
        })
        .unwrap_or_else(|| image_name.to_string())
}

pub fn get_type(
    hashes: &HashMap<String, NodeType>,
    hash: &str,
    image_name: &str,
    container_name: &str,
) -> Option<NodeType> {
    let node_type = hashes
        .get(hash)
        .copied()
        .or_else(|| NodeType::from_image(&extract_image_name(image_name)))
        .or_else(|| NodeType::from_default_container_name(container_name.trim_start_matches('/')));
    if node_type.is_none() {
        warn!("No node type found for {}", image_name);
    }
    node_type
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_image_name() {
        let test_cases = vec![
            // Standard registry cases
            ("docker.io/ubuntu:latest", "ubuntu:latest"),
            ("gcr.io/project/image:v1", "project/image:v1"),
            ("ghcr.io/owner/repo:tag", "owner/repo:tag"),
            ("public.ecr.aws/image:1.0", "image:1.0"),
            // Edge cases
            ("ubuntu:latest", "ubuntu:latest"), // No registry
            ("", ""),                           // Empty string
            ("repository.chainbase.com/", ""),  // Just registry
            // Multiple registry-like strings
            ("gcr.io/docker.io/image", "image"), // Should match first registry
            // With and without tags
            ("docker.io/image", "image"),
            ("docker.io/org/image:latest", "org/image:latest"),
            // Special characters
            ("docker.io/org/image@sha256:123", "org/image@sha256:123"),
            ("docker.io/org/image_name", "org/image_name"),
        ];

        for (input, expected) in test_cases {
            assert_eq!(
                extract_image_name(input),
                expected.to_string(),
                "Failed on input: {}",
                input
            );
        }
    }
}
