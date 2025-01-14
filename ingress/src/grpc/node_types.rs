use std::collections::HashMap;

use ivynet_docker::RegistryType;
use ivynet_node_type::{AltlayerType, MachType, NodeType};

pub fn get_node_type(
    hashes: &Option<HashMap<String, NodeType>>,
    hash: &str,
    image_name: &str,
    container_name: &str,
) -> Option<NodeType> {
    let cleaned_container_name = container_name.trim_start_matches('/');

    let extracted_image_name = extract_image_name(image_name);
    NodeType::from_image(&extracted_image_name)
        .and_then(|nt| handle_altlayer_unknown(nt, cleaned_container_name))
        .or_else(|| {
            hashes
                .as_ref()
                .and_then(|h| h.get(hash))
                .copied()
                .and_then(|nt| handle_altlayer_unknown(nt, cleaned_container_name))
        })
        .or_else(|| NodeType::from_default_container_name(cleaned_container_name))
}

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

fn handle_altlayer_unknown(nt: NodeType, container_name: &str) -> Option<NodeType> {
    match nt {
        NodeType::Altlayer(AltlayerType::Unknown) | NodeType::AltlayerMach(MachType::Unknown) => {
            NodeType::from_default_container_name(container_name)
        }
        _ => Some(nt),
    }
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
