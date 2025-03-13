use api::{
    config::Config, error::BackendError, get_node_version_hashes, http, telemetry::start_tracing,
};
use chrono::DateTime;
use clap::Parser as _;
use ethers::types::Chain;
use ivynet_database::{
    self,
    avs_version::DbAvsVersionData,
    configure,
    data::avs_version::{find_latest_avs_version, VersionType},
    utils::try_parse_chain,
};
use ivynet_node_type::{ActiveSet, AltlayerType, MachType, NodeType};
use sqlx::PgPool;
use strum::IntoEnumIterator;
use tracing::{debug, error, info, warn};

#[tokio::main]
async fn main() -> Result<(), BackendError> {
    let config = Config::parse();

    start_tracing(config.log_level)?;

    let pool = configure(&config.db_uri, config.migrate).await?;

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
    } else if config.update_node_data_versions {
        let node_types = NodeType::all_known_with_repo();
        ivynet_database::DbAvsVersionData::delete_avses_from_avs_version_data(&pool, &node_types)
            .await?;
        update_node_data_versions(&node_types, &pool, &Chain::Mainnet).await?;
        update_node_data_versions(&node_types, &pool, &Chain::Holesky).await?;
        return Ok(());
    } else if config.delete_old_logs {
        Ok(ivynet_database::log::ContainerLog::delete_old_logs(&pool).await?)
    } else {
        let cache = memcache::connect(config.cache_url.to_string())?;
        http::serve(
            pool.clone(),
            cache,
            config.root_url,
            config.sendgrid_api_key,
            config.sendgrid_from,
            config.org_verification_template,
            config.user_verification_template,
            config.pass_reset_template,
            config.http_port,
        )
        .await?;

        Ok(())
    }
}

async fn add_account(pool: &PgPool, info: &str) -> Result<(), BackendError> {
    let org_data = info.split('/').collect::<Vec<_>>();
    if org_data.len() == 2 {
        let cred_data = org_data[0].split(':').collect::<Vec<_>>();
        if cred_data.len() == 2 {
            println!("Creating organization {} with user {}", org_data[1], cred_data[0]);
            let org = ivynet_database::Organization::new(pool, org_data[1], true).await?;
            org.attach_admin(pool, cred_data[0], cred_data[1]).await?;
        } else {
            println!("Try testuser@ivynet.dev:test1234/testorg");
        }
    } else {
        let cred_data = info.split(':').collect::<Vec<_>>();
        if cred_data.len() == 2 {
            println!("Creating user without organization on org 1");
            let org = ivynet_database::Organization::get(pool, 1).await?;
            org.attach_admin(pool, cred_data[0], cred_data[1]).await?;
        } else {
            println!("Try testuser@ivynet.dev:test1234");
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
            ivynet_database::AvsVersionHash::add_version(
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

// TODO: Guards around setters. You should not be able to set a version that does not exist in the
// db.

async fn set_avs_version(pool: &sqlx::PgPool, avs_data: &str) -> Result<(), BackendError> {
    let avs_data = avs_data.split(':').collect::<Vec<_>>();
    if avs_data.len() < 3 {
        return Err(BackendError::InvalidSetAvsVersionData);
    }
    let node_type = NodeType::from(avs_data[0]);
    let chain = try_parse_chain(avs_data[1]).map_err(|_| BackendError::InvalidChain)?;
    let version = avs_data[2];
    let digest = ivynet_database::AvsVersionHash::get_digest_for_version(
        pool,
        &node_type.to_string(),
        version,
    )
    .await?;

    println!("Setting version {:?} for avs {:?} on {:?}", version, node_type, chain);
    DbAvsVersionData::set_avs_version(pool, &node_type, &chain, version, &digest).await?;
    Ok(())
}

async fn set_breaking_change_version(
    pool: &sqlx::PgPool,
    avs_data: &str,
) -> Result<(), BackendError> {
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

async fn add_node_version_hashes(pool: &PgPool) -> Result<(), BackendError> {
    let all_known_with_repo = NodeType::all_known_with_repo();
    ivynet_database::AvsVersionHash::delete_avses_from_avs_version_hash(pool, &all_known_with_repo)
        .await?;
    let registry_tags = get_node_version_hashes().await?;
    info!("Adding {} total node version hashes", registry_tags.len());
    for (entry, tags) in registry_tags {
        let name = entry.to_string();
        match VersionType::from(&entry) {
            VersionType::SemVer => {
                info!("Adding SemVer version hashes for {}", name);
                for (tag, digest) in tags {
                    if !tag.is_empty() && !digest.is_empty() {
                        match ivynet_database::AvsVersionHash::add_version(
                            pool, &entry, &digest, &tag,
                        )
                        .await
                        {
                            Ok(_) => debug!("Added {}:{}:{}", name, tag, digest),
                            Err(e) => debug!("Failed to add {}:{}:{} | {}", name, tag, digest, e),
                        };
                    } else {
                        error!("Dropping adding an empty entry (tag '{tag}' digest '{digest}')");
                    }
                }
            }
            VersionType::FixedVer | VersionType::HybridVer => {
                debug!("Updating fixed and hybrid version hashes for {}", name);
                for (tag, digest) in tags {
                    if !tag.is_empty() && !digest.is_empty() {
                        match ivynet_database::AvsVersionHash::update_version(
                            pool, &entry, &digest, &tag,
                        )
                        .await
                        {
                            Ok(_) => debug!("Updated {}:{}:{}", name, tag, digest),
                            Err(e) => warn!("Failed to update {}:{}:{} | {}", name, tag, digest, e),
                        };
                    } else {
                        error!(
                            "Dropping updating to an empty entry (tag '{tag}' digest '{digest}')"
                        );
                    }
                }
            }
            VersionType::LocalOnly => {
                info!("Skipping local only node type {}", name);
                continue;
            }
            VersionType::OptInOnly => {
                info!("Skipping opt-in only node type {}", name);
                continue;
            }
        }
    }

    Ok(())
}

async fn update_node_data_versions(
    node_types: &Vec<NodeType>,
    pool: &PgPool,
    chain: &Chain,
) -> Result<(), BackendError> {
    info!("Updating node data versions for {:?}", chain);
    for node_type in node_types {
        match (node_type, chain) {
            (NodeType::Gasp, _) => continue,
            (NodeType::K3LabsAvsHolesky, Chain::Mainnet) => continue,
            (NodeType::K3LabsAvs, Chain::Holesky) => continue,
            (NodeType::OpenLayerHolesky, Chain::Mainnet) => continue,
            (NodeType::OpenLayerMainnet, Chain::Holesky) => continue,
            (NodeType::Altlayer(altlayer_type), _) => match altlayer_type {
                AltlayerType::Unknown => {
                    let (tag, digest) = find_latest_avs_version(pool, node_type, chain).await?;
                    for altlayer_type in AltlayerType::iter() {
                        ivynet_database::DbAvsVersionData::set_avs_version(
                            pool,
                            &NodeType::Altlayer(altlayer_type),
                            chain,
                            &tag,
                            &digest,
                        )
                        .await?;
                    }
                }
                _ => continue,
            },
            (NodeType::AltlayerMach(mach_type), _) => match mach_type {
                MachType::Unknown => {
                    let (tag, digest) = find_latest_avs_version(pool, node_type, chain).await?;
                    for mach_type in MachType::iter() {
                        ivynet_database::DbAvsVersionData::set_avs_version(
                            pool,
                            &NodeType::AltlayerMach(mach_type),
                            chain,
                            &tag,
                            &digest,
                        )
                        .await?;
                    }
                }
                _ => continue,
            },
            (NodeType::DittoNetwork(active_set), _) => match active_set {
                ActiveSet::Unknown => {
                    let (tag, digest) = find_latest_avs_version(pool, node_type, chain).await?;
                    for protocol in ActiveSet::iter() {
                        ivynet_database::DbAvsVersionData::set_avs_version(
                            pool,
                            &NodeType::DittoNetwork(protocol),
                            chain,
                            &tag,
                            &digest,
                        )
                        .await?;
                    }
                }
                _ => continue,
            },
            (NodeType::Bolt(active_set), _) => match active_set {
                ActiveSet::Unknown => {
                    let (tag, digest) = find_latest_avs_version(pool, node_type, chain).await?;
                    for protocol in ActiveSet::iter() {
                        ivynet_database::DbAvsVersionData::set_avs_version(
                            pool,
                            &NodeType::Bolt(protocol),
                            chain,
                            &tag,
                            &digest,
                        )
                        .await?;
                    }
                }
                _ => continue,
            },
            (NodeType::Hyperlane(active_set), _) => match active_set {
                ActiveSet::Unknown => {
                    let (tag, digest) = find_latest_avs_version(pool, node_type, chain).await?;
                    for protocol in ActiveSet::iter() {
                        ivynet_database::DbAvsVersionData::set_avs_version(
                            pool,
                            &NodeType::Hyperlane(protocol),
                            chain,
                            &tag,
                            &digest,
                        )
                        .await?;
                    }
                }
                _ => continue,
            },
            _ => {
                let (tag, digest) = find_latest_avs_version(pool, node_type, chain).await?;
                ivynet_database::DbAvsVersionData::set_avs_version(
                    pool, node_type, chain, &tag, &digest,
                )
                .await?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use ethers::types::Chain;
    use ivynet_database::{data::avs_version::find_latest_avs_version, DbAvsVersionData};
    use ivynet_node_type::NodeType;
    use sqlx::PgPool;

    use crate::update_node_data_versions;

    async fn test_update_nodes(
        pool: &PgPool,
        node_types: Vec<NodeType>,
        chain: &Chain,
    ) -> Result<(), sqlx::Error> {
        let mut types_hashmap = HashMap::new();

        for t in &node_types {
            types_hashmap.insert(
                t,
                DbAvsVersionData::get_avs_version_with_chain(pool, t, chain).await.unwrap(),
            );
        }

        update_node_data_versions(&node_types, pool, chain).await.unwrap();

        let tags = {
            let mut map = HashMap::new();
            for t in node_types.iter() {
                let (tag, digest) = find_latest_avs_version(pool, t, chain).await.unwrap();
                map.insert(t, (tag, digest));
            }
            map
        };

        let mut new_versions_map = HashMap::new();
        for t in node_types.iter() {
            new_versions_map.insert(
                t,
                DbAvsVersionData::get_avs_version_with_chain(pool, t, chain).await.unwrap(),
            );
        }

        for (t, (tag, _digest)) in tags.iter() {
            assert_ne!(types_hashmap.get(t), new_versions_map.get(t));
            assert_ne!(
                types_hashmap.get(t).unwrap().clone().unwrap().vd.latest_version,
                tag.to_owned()
            );
            assert_eq!(
                new_versions_map.get(t).unwrap().clone().unwrap().vd.latest_version,
                tag.to_owned()
            );
        }
        Ok(())
    }

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../fixtures/setup_altlayer_node_data_versions.sql")
    )]
    async fn test_update_node_data_versions_testnet(pool: PgPool) -> Result<(), sqlx::Error> {
        let node_types: Vec<_> = NodeType::all_machtypes().into_iter().collect();
        test_update_nodes(&pool, node_types, &Chain::Holesky).await?;

        let node_types: Vec<_> = NodeType::all_altlayertypes().into_iter().collect();
        test_update_nodes(&pool, node_types, &Chain::Holesky).await?;
        Ok(())
    }

    #[ignore]
    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../fixtures/setup_altlayer_node_data_versions.sql")
    )]
    async fn test_update_node_data_versions_mainnet(pool: PgPool) -> Result<(), sqlx::Error> {
        let node_types: Vec<_> = NodeType::all_machtypes().into_iter().collect();
        test_update_nodes(&pool, node_types, &Chain::Mainnet).await?;

        let node_types: Vec<_> = NodeType::all_altlayertypes().into_iter().collect();
        test_update_nodes(&pool, node_types, &Chain::Mainnet).await?;
        Ok(())
    }
}
