use ivynet_core::{docker::NodeRegistryEntry, ethers::types::Chain};
use tracing::info;

use crate::{db::AvsVersionHash, error::BackendError};

pub enum VersionType {
    /// For node types which continually update a "latest" tag, without semver
    Latest,
    SemVer,
}

impl From<&NodeRegistryEntry> for VersionType {
    fn from(node_type: &NodeRegistryEntry) -> Self {
        match node_type {
            NodeRegistryEntry::EigenDA => VersionType::SemVer,
            NodeRegistryEntry::LagrangeZKCoprocessor => VersionType::SemVer,
            NodeRegistryEntry::LagrangeStateCommittee => VersionType::SemVer,
            NodeRegistryEntry::Ava => VersionType::SemVer,
            NodeRegistryEntry::Eoracle => todo!(),
            NodeRegistryEntry::K3Labs => VersionType::Latest,
            // NodeRegistryEntry::Hyperlane => todo!(),
            _ => todo!(),
        }
    }
}
/// Pulls tags from the db, selects versioning type depending on the node type, and returns the
/// latest version. This
pub async fn find_latest_avs_version(
    pool: &sqlx::PgPool,
    registry_entry: &NodeRegistryEntry,
) -> Result<String, BackendError> {
    let avs_name = registry_entry.registry_entry().name;

    // get tags from db
    let version_list = AvsVersionHash::get_all_tags_for_type(pool, &avs_name).await?;
    info!("Found {} tags for {}", version_list.len(), avs_name);

    let tag = match VersionType::from(registry_entry) {
        VersionType::Latest => "latest".to_string(),
        VersionType::SemVer => {
            // If a version type is semver, we sanitize the list, discarding the other
            // elements.
            println!("raw version list: {:?}", version_list);
            let version_vec = version_list
                .iter()
                .filter_map(|tag| {
                    // sanitize the tag via regex
                    let sanitized =
                        regex::Regex::new(SEMVER_REGEX).unwrap().find(tag).map(|m| m.as_str());
                    println!("sanitized: {:?}", sanitized);
                    let semver_tag = semver::Version::parse(sanitized?).ok()?;
                    Some((tag, semver_tag))
                })
                .collect::<Vec<_>>();

            let latest = version_vec.iter().max_by_key(|(_, v)| v);
            let latest = latest.ok_or(BackendError::NoVersionsFound)?;
            latest.0.to_string()
        }
    };
    Ok(tag)
}

/// Regex for semver parsing
/// Taken from `https://semver.org/#is-there-a-suggested-regular-expression-regex-to-check-a-semver-string`
/// Modified to not necessarily start from the beginning of the line, allowing for matching against
/// tags that may have a nonstandard prefix such as `v`.
const SEMVER_REGEX: &str = r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$";

#[cfg(test)]
mod avs_version_tests {
    use super::*;
    use sqlx::PgPool;

    #[test]
    fn test_semver_regex() {
        let re = regex::Regex::new(SEMVER_REGEX).unwrap();
        let valid = vec![
            "1.0.0",
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-0.3.7",
            "1.0.0-x.7.z.92",
            "1.0.0+20130313144700",
            "1.0.0-beta+exp.sha.5114f85",
            "v1.0.0",
            "agent-1.0.0",
        ];

        for v in valid {
            assert!(re.is_match(v));
        }
    }

    // TODO: These tests need to be more abstract and run over dummy data instead of live db data.

    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    fn test_eigenda_version_parsing(pool: PgPool) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        println!("{:#?}", pool.options());
        let node_registry_entry = NodeRegistryEntry::EigenDA;
        let _ = find_latest_avs_version(&pool, &node_registry_entry).await?;
        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    fn test_ava_version_parsing(pool: PgPool) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        let node_registry_entry = NodeRegistryEntry::Ava;
        let _ = find_latest_avs_version(&pool, &node_registry_entry).await?;
        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    fn test_k3labs_version_parsing(pool: PgPool) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        let node_registry_entry = NodeRegistryEntry::K3Labs;
        let _ = find_latest_avs_version(&pool, &node_registry_entry).await?;
        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    fn test_lagrange_zk_version_parsing(
        pool: PgPool,
    ) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        let node_registry_entry = NodeRegistryEntry::LagrangeZKCoprocessor;
        let _ = find_latest_avs_version(&pool, &node_registry_entry).await?;
        Ok(())
    }
}
