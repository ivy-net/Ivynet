use ivynet_core::{docker::NodeRegistryEntry, ethers::types::Chain, node_type::NodeType};
use tracing::info;

use crate::{db::AvsVersionHash, error::BackendError};

pub enum VersionType {
    /// For node types which continually update a "latest" tag, without semver
    Latest,
    /// For node types which adhere to semver spec. Initializes to None when being derived from
    /// NodeRegistryEntry, and is populated when the latest version is fetched from the db.
    SemVer(Option<semver::Version>),
}

impl From<&NodeRegistryEntry> for VersionType {
    fn from(node_type: &NodeRegistryEntry) -> Self {
        match node_type {
            NodeRegistryEntry::EigenDA => VersionType::SemVer(None),
            NodeRegistryEntry::LagrangeZKCoprocessor => todo!(),
            NodeRegistryEntry::LagrangeStateCommittee => todo!(),
            NodeRegistryEntry::Ava => VersionType::SemVer(None),
            NodeRegistryEntry::Eoracle => todo!(),
            NodeRegistryEntry::K3Labs => VersionType::Latest,
            // NodeRegistryEntry::Hyperlane => todo!(),
            _ => todo!(),
        }
    }
}

pub async fn find_latest_avs_version(
    pool: &sqlx::PgPool,
    registry_entry: &NodeRegistryEntry,
    chain: Option<&Chain>,
) -> Result<Option<VersionType>, BackendError> {
    let avs_name = registry_entry.registry_entry().name;
    println!("Fetching tags for {}", avs_name);

    // get tags from db
    let version_list = AvsVersionHash::get_all_tags_for_type(pool, &avs_name).await?;
    info!("Found {} tags for {}", version_list.len(), avs_name);

    let latest = match VersionType::from(registry_entry) {
        VersionType::Latest => {
            println!("Versioning for {} is Latest", avs_name);
            println!("{:?}", version_list);
        }
        VersionType::SemVer(_) => {
            let version_vec = version_list
                .iter()
                .map(|tag| semver::Version::parse(tag).unwrap())
                .collect::<Vec<_>>();
            println!("Versioning for {} is SemVer", avs_name);
            println!("{:?}", version_vec);
        }
    };

    Ok(None)
}

#[cfg(test)]
mod avs_version_tests {
    use super::*;
    use sqlx::PgPool;

    // TODO: These tests need to be more abstract and run over dummy data instead of live db data.

    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    fn test_eigenda_version_parsing(pool: PgPool) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        println!("{:#?}", pool.options());
        let node_registry_entry = NodeRegistryEntry::EigenDA;
        let chain = None;
        let _ = find_latest_avs_version(&pool, &node_registry_entry, chain).await?;
        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    fn test_ava_version_parsing(pool: PgPool) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        let node_registry_entry = NodeRegistryEntry::Ava;
        let chain = None;
        let _ = find_latest_avs_version(&pool, &node_registry_entry, chain).await?;
        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    fn test_k3labs_version_parsing(pool: PgPool) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        let node_registry_entry = NodeRegistryEntry::K3Labs;
        let chain = None;
        let _ = find_latest_avs_version(&pool, &node_registry_entry, chain).await?;
        Ok(())
    }
}
