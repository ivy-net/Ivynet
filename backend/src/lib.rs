use std::collections::HashMap;

use data::avs_version::{extract_semver, VersionType};
use error::BackendError;
use futures::future::join_all;
use ivynet_core::{docker::DockerRegistry, node_type::NodeType};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use tracing::{error, info, warn};

pub mod config;
pub mod data;
pub mod db;
pub mod error;
pub mod grpc;
pub mod http;
pub mod telemetry;

pub async fn get_node_version_hashes(
) -> Result<HashMap<NodeType, Vec<(String, String)>>, BackendError> {
    let mut registry_tags = HashMap::new();

    for entry in NodeType::all_known() {
        let client = DockerRegistry::from_node_type(&entry).await?;
        info!("Requesting tags for image {}", entry.default_repository()?);

        let tags = client.get_tags().await?;
        let tag_digests = fetch_tag_digests(&client, tags).await?;

        registry_tags.insert(entry, tag_digests);
    }
    Ok(registry_tags)
}

#[allow(dead_code)]
async fn get_filtered_tags(
    client: &DockerRegistry,
    node_type: &NodeType,
) -> Result<Vec<String>, BackendError> {
    let mut tags = client.get_tags().await?;
    let start_len = tags.len();

    if VersionType::from(node_type) == VersionType::SemVer {
        tags = tags.into_par_iter().filter(|tag| extract_semver(tag).is_some()).collect();
    }

    info!("Filtered {} tags: {} -> {}", node_type, start_len, tags.len());

    Ok(tags)
}

async fn fetch_tag_digests(
    client: &DockerRegistry,
    tags: Vec<String>,
) -> Result<Vec<(String, String)>, BackendError> {
    let mut tag_map: HashMap<String, String> =
        tags.iter().map(|tag| (tag.clone(), String::new())).collect();

    for batch in tags.chunks(10) {
        process_digest_batch(client, batch, &mut tag_map).await?;
    }

    Ok(tag_map.into_iter().collect())
}

async fn process_digest_batch(
    client: &DockerRegistry,
    batch: &[String],
    tag_map: &mut HashMap<String, String>,
) -> Result<(), BackendError> {
    let mut retries = 0;
    let max_retries = 4;
    let mut delay = tokio::time::Duration::from_secs(1);

    let results = loop {
        let futures: Vec<(&String, _)> =
            batch.iter().map(|tag| (tag, client.get_tag_digest(tag))).collect();

        let results = join_all(futures.into_iter().map(|(_, f)| f)).await;

        match results {
            futures if futures.iter().all(|r| r.is_ok()) => {
                break batch.iter().zip(futures);
            }
            results if retries >= max_retries => {
                error!("Max retries reached for batch");
                break batch.iter().zip(results);
            }
            _ => {
                warn!("Retrying batch after delay of {}s", delay.as_secs());
                tokio::time::sleep(delay).await;
                delay *= 2;
                retries += 1;
                continue;
            }
        }
    };

    update_tag_map(results, tag_map);
    Ok(())
}

fn update_tag_map<'a, T: std::fmt::Debug>(
    results: impl Iterator<Item = (&'a String, Result<Option<String>, T>)>,
    tag_map: &mut HashMap<String, String>,
) {
    for (tag, res) in results {
        match res {
            Ok(Some(digest)) => {
                tag_map.insert(tag.to_string(), digest);
            }
            Ok(None) => {
                error!("Failed to get digest for tag. No digest found for {}", tag);
            }
            Err(e) => {
                error!("Failed to get digest for {}: {:?}", tag, e);
            }
        }
    }
}
