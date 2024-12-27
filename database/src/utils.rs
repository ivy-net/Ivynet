use ethers::types::Chain;
use ivynet_core::utils::try_parse_chain;
use ivynet_node_type::NodeType;
use sqlx::{types::chrono::DateTime, PgPool};
use tracing::{debug, info, warn};

use crate::{
    avs_version::{find_latest_avs_version, VersionType},
    db::{self, DbAvsVersionData},
    docker::get_node_version_hashes,
    error::DatabaseError,
};

pub async fn add_account(pool: &PgPool, org: &str) -> Result<(), DatabaseError> {
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

pub async fn add_version_hash(pool: &PgPool, version: &str) -> Result<(), DatabaseError> {
    let version_data = version.split('=').collect::<Vec<_>>();
    if version_data.len() == 2 {
        let avs_data = version_data[0].split(':').collect::<Vec<_>>();
        if avs_data.len() == 2 {
            println!(
                "Adding new version ({}) for avs {} with hash = {}",
                avs_data[0], avs_data[1], version_data[1]
            );
            db::AvsVersionHash::add_version(
                pool,
                &NodeType::from(avs_data[1]),
                version_data[1],
                avs_data[0],
            )
            .await?;
        }
    }
    Ok(())
}

pub async fn set_avs_version(pool: &sqlx::PgPool, avs_data: &str) -> Result<(), DatabaseError> {
    let avs_data = avs_data.split(':').collect::<Vec<_>>();
    if avs_data.len() < 3 {
        return Err(DatabaseError::InvalidSetAvsVersionData);
    }
    let node_type = NodeType::from(avs_data[0]);
    let chain = try_parse_chain(avs_data[1]).map_err(|_| DatabaseError::InvalidChain)?;
    let version = avs_data[2];
    let digest =
        db::AvsVersionHash::get_digest_for_version(pool, &node_type.to_string(), version).await?;

    println!("Setting version {:?} for avs {:?} on {:?}", version, node_type, chain);
    DbAvsVersionData::set_avs_version(pool, &node_type, &chain, version, &digest).await?;
    Ok(())
}

pub async fn set_breaking_change_version(
    pool: &sqlx::PgPool,
    avs_data: &str,
) -> Result<(), DatabaseError> {
    let avs_data = avs_data.split(':').collect::<Vec<_>>();
    let name = NodeType::from(avs_data[0]);
    let chain = try_parse_chain(avs_data[1]).expect("Cannot parse chain");
    let version = avs_data[2];
    let timestamp = avs_data[3].parse::<i64>().expect("Cannot parse datetime") / 1000;
    let datetime = DateTime::from_timestamp(timestamp, 0).expect("Invalid timestamp").naive_utc();

    println!(
        "Setting breaking change version {:?} at {:?} for avs {:?} on {:?}",
        version, datetime, name, chain
    );
    DbAvsVersionData::set_breaking_change_version(pool, &name, &chain, version, &datetime).await?;
    Ok(())
}

pub async fn add_node_version_hashes(pool: &PgPool) -> Result<(), DatabaseError> {
    let registry_tags = get_node_version_hashes().await?;
    info!("Adding {} total node version hashes", registry_tags.len());
    for (entry, tags) in registry_tags {
        let name = entry.to_string();
        match VersionType::from(&entry) {
            VersionType::SemVer => {
                info!("Adding SemVer version hashes for {}", name);
                for (tag, digest) in tags {
                    match db::AvsVersionHash::add_version(pool, &entry, &digest, &tag).await {
                        Ok(_) => debug!("Added {}:{}:{}", name, tag, digest),
                        Err(e) => warn!("Failed to add {}:{}:{} | {}", name, tag, digest, e),
                    };
                }
            }
            VersionType::FixedVer | VersionType::HybridVer => {
                debug!("Updating fixed and hybrid version hashes for {}", name);
                for (tag, digest) in tags {
                    match db::AvsVersionHash::update_version(pool, &entry, &digest, &tag).await {
                        Ok(_) => debug!("Updated {}:{}:{}", name, tag, digest),
                        Err(e) => warn!("Failed to update {}:{}:{} | {}", name, tag, digest, e),
                    };
                }
            }
        }
    }
    Ok(())
}

pub async fn update_node_data_versions(pool: &PgPool, chain: &Chain) -> Result<(), DatabaseError> {
    info!("Updating node data versions for {:?}", chain);
    let node_types = NodeType::all_known();
    for node in node_types {
        if node == NodeType::LagrangeZkWorkerHolesky && chain == &Chain::Mainnet {
            continue;
        }
        if node == NodeType::LagrangeZkWorkerMainnet && chain == &Chain::Holesky {
            continue;
        }
        let (tag, digest) = find_latest_avs_version(pool, &node).await?;
        db::DbAvsVersionData::set_avs_version(pool, &node, chain, &tag, &digest).await?;
    }
    Ok(())
}
