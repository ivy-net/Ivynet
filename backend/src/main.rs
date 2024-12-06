use std::{collections::HashMap, sync::Arc};

use chrono::DateTime;
use clap::Parser as _;
use ivynet_backend::{
    config::Config,
    db::{self, avs_version::DbAvsVersionData, configure},
    error::BackendError,
    grpc, http,
    telemetry::start_tracing,
};
use ivynet_core::{
    docker::{DockerRegistry, NodeRegistryEntry},
    node_type::NodeType,
    utils::try_parse_chain,
};
use semver::Version;
use sqlx::PgPool;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), BackendError> {
    let config = Config::parse();

    start_tracing(config.log_level)?;

    let pool = Arc::new(configure(&config.db_uri, config.migrate).await?);

    if let Some(organization) = config.add_organization {
        Ok(add_account(&pool, &organization).await?)
    } else if let Some(avs_data) = config.set_avs_version {
        Ok(set_avs_version(&pool, &avs_data).await?)
    } else if let Some(avs_data) = config.add_avs_version_hash {
        Ok(add_version_hash(&pool, &avs_data).await?)
    } else if let Some(avs_data) = config.set_breaking_change_version {
        Ok(set_breaking_change_version(&pool, &avs_data).await?)
    } else if config.add_node_version_hashes {
        Ok(add_node_version_hashes(&pool).await?)
    } else {
        let cache = memcache::connect(config.cache_url.to_string())?;
        let http_service = http::serve(
            pool.clone(),
            cache,
            config.root_url,
            config.sendgrid_api_key,
            config.sendgrid_from,
            config.org_verification_template,
            config.user_verification_template,
            config.pass_reset_template,
            config.http_port,
        );
        let grpc_service =
            grpc::serve(pool, config.grpc_tls_cert, config.grpc_tls_key, config.grpc_port);

        tokio::select! {
            e = http_service => error!("HTTP server stopped. Reason {e:?}"),
            e = grpc_service => error!("Executor has stopped. Reason: {e:?}"),
        }

        Ok(())
    }
}

#[derive(clap::Parser, Debug, Clone)]
pub enum BackendCommands {
    #[command(
        name = "add-version-hashes",
        about = "Add version hashes for all nodes in the registry to backend db"
    )]
    AddNodeVersionHashes,
}

async fn add_account(pool: &PgPool, org: &str) -> Result<(), BackendError> {
    let org_data = org.split('/').collect::<Vec<_>>();
    if org_data.len() == 2 {
        let cred_data = org_data[0].split(':').collect::<Vec<_>>();
        if cred_data.len() == 2 {
            println!("Creating organization {} with user {}", org_data[1], cred_data[0]);
            let org = db::Organization::new(pool, org_data[1], true).await?;
            org.attach_admin(pool, cred_data[0], cred_data[1]).await?;
        }
    }
    Ok(())
}

async fn add_version_hash(pool: &PgPool, version: &str) -> Result<(), BackendError> {
    let version_data = version.split('=').collect::<Vec<_>>();
    if version_data.len() == 2 {
        let avs_data = version_data[0].split(':').collect::<Vec<_>>();
        if avs_data.len() == 2 {
            println!(
                "Adding new version ({}) for avs {} with hash = {}",
                avs_data[0], avs_data[1], version_data[1]
            );
            db::AvsVersionHash::add_version(pool, avs_data[1], version_data[1], avs_data[0])
                .await?;
        }
    }
    Ok(())
}

async fn set_avs_version(pool: &sqlx::PgPool, avs_data: &str) -> Result<(), BackendError> {
    let avs_data = avs_data.split(':').collect::<Vec<_>>();
    let name = NodeType::from(avs_data[0]);
    let chain = try_parse_chain(avs_data[1]).expect("Cannot parse chain");
    let version = Version::parse(avs_data[2]).expect("Cannot parse version");

    println!("Setting version {:?} for avs {:?} on {:?}", version, name, chain);
    DbAvsVersionData::set_avs_version(pool, &name, &chain, &version).await?;
    Ok(())
}

async fn set_breaking_change_version(
    pool: &sqlx::PgPool,
    avs_data: &str,
) -> Result<(), BackendError> {
    let avs_data = avs_data.split(':').collect::<Vec<_>>();
    let name = NodeType::from(avs_data[0]);
    let chain = try_parse_chain(avs_data[1]).expect("Cannot parse chain");
    let version = Version::parse(avs_data[2]).expect("Cannot parse breaking change version");
    let timestamp = avs_data[3].parse::<i64>().expect("Cannot parse datetime") / 1000;
    let datetime = DateTime::from_timestamp(timestamp, 0).expect("Invalid timestamp").naive_utc();

    println!(
        "Setting breaking change version {:?} at {:?} for avs {:?} on {:?}",
        version, datetime, name, chain
    );
    DbAvsVersionData::set_breaking_change_version(pool, &name, &chain, &version, &datetime).await?;
    Ok(())
}

async fn add_node_version_hashes(pool: &PgPool) -> Result<(), BackendError> {
    let registry_tags = get_node_version_hashes().await?;
    for (entry, tags) in registry_tags {
        info!("Adding version hashes for {}", entry.registry_entry().name);
        for (tag, digest) in tags {
            let name = entry.registry_entry().name;
            match db::AvsVersionHash::add_version(pool, &name, &digest, &tag).await {
                Ok(_) => {}
                Err(e) => warn!("Failed to add {}:{}:{} | {}", name, tag, digest, e),
            };
        }
    }
    Ok(())
}

//  Resulting hashmap returns a vec - tuple of (tag, digest), with digest as an empty string if not found
async fn get_node_version_hashes(
) -> Result<HashMap<NodeRegistryEntry, Vec<(String, String)>>, BackendError> {
    let mut registry_tags = HashMap::new();

    for entry in NodeRegistryEntry::all() {
        let client = DockerRegistry::from_node_registry_entry(&entry).await?;
        info!("Requesting tags for image {}", entry.registry_entry().image);
        let tags = client.get_tags().await?;

        let mut num_valid_digests = 0;
        let mut tag_digests = Vec::new();

        let tags_len = tags.len();
        for tag in tags {
            let digest = client.get_tag_digest(&tag).await?.unwrap_or_default();
            if !digest.is_empty() {
                num_valid_digests += 1;
            }
            tag_digests.push((tag, digest));
        }
        info!("Found {} valid digests for {} tags", num_valid_digests, tags_len);
        registry_tags.insert(entry, tag_digests);
    }
    Ok(registry_tags)
}
