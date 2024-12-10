use ivynet_core::node_type::NodeType;
use serde::{Deserialize, Serialize};
use tracing::info;
use utoipa::ToSchema;

use crate::{db::AvsVersionHash, error::BackendError};

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub enum VersionType {
    SemVer,
    /// For node types with fixed docker versioning tags, such as `latest` or `holesky`
    FixedVer,
}

// TODO: This is really messy, should probably live in core but has a ToSchema dep
impl From<&NodeType> for VersionType {
    fn from(node_type: &NodeType) -> Self {
        match node_type {
            NodeType::EigenDA => VersionType::SemVer,
            NodeType::LagrangeZkWorkerHolesky => VersionType::FixedVer,
            NodeType::LagrangeZkWorkerMainnet => VersionType::FixedVer,
            //NodeType::LagrangeStateCommittee => VersionType::SemVer,
            NodeType::AvaProtocol => VersionType::SemVer,
            NodeType::EOracle => VersionType::SemVer,
            NodeType::K3LabsAvs => VersionType::FixedVer,
            NodeType::Predicate => VersionType::SemVer,
            // NodeType::Hyperlane => todo!(),
            _ => todo!("{:?} version type not yet implemented", node_type.to_string()),
        }
    }
}

impl VersionType {
    pub fn fixed_name(node_type: &NodeType) -> Option<&'static str> {
        match node_type {
            NodeType::LagrangeZkWorkerHolesky => Some("holesky"),
            NodeType::LagrangeZkWorkerMainnet => Some("mainnet"),
            NodeType::K3LabsAvs => Some("latest"),
            _ => None,
        }
    }
}

/// Pulls tags from the db, selects versioning type depending on the node type, and returns the
/// latest version via `(tag, digest).`
pub async fn find_latest_avs_version(
    pool: &sqlx::PgPool,
    node_type: &NodeType,
) -> Result<(String, String), BackendError> {
    let avs_name = node_type.to_string();

    // get tags from db
    let version_list = AvsVersionHash::get_all_for_type(pool, &avs_name).await?;
    info!("Found {} tags for {}", version_list.len(), avs_name);

    let (tag, digest) = match VersionType::from(node_type) {
        VersionType::FixedVer => {
            let tag = VersionType::fixed_name(node_type).unwrap().to_string();
            let digest = version_list
                .iter()
                .find(|version_data| version_data.version == tag)
                .ok_or(BackendError::NoVersionsFound)?
                .hash
                .clone();
            (tag, digest)
        }
        VersionType::SemVer => {
            // If a version type is semver, we sanitize the list, discarding the other
            // elements.
            let version_vec = version_list
                .iter()
                .filter_map(|version_data| {
                    let raw_tag = version_data.version.clone();
                    let digest = version_data.hash.clone();
                    // sanitize the tag via regex
                    let semver_tag = extract_semver(&raw_tag)?;
                    Some((semver_tag, raw_tag, digest))
                })
                .collect::<Vec<_>>();

            // filter prerelease versions
            let version_vec =
                version_vec.into_iter().filter(|(v, _, _)| v.pre.is_empty()).collect::<Vec<_>>();

            let latest = version_vec.iter().max_by_key(|(v, _, _)| v);
            let latest = latest.ok_or(BackendError::NoVersionsFound)?;
            (latest.1.to_string(), latest.2.to_string())
        }
    };
    Ok((tag, digest))
}

pub fn extract_semver(tag: &str) -> Option<semver::Version> {
    let sanitized = regex::Regex::new(SEMVER_REGEX).unwrap().find(tag).map(|m| m.as_str());
    sanitized.and_then(|s| semver::Version::parse(s).ok())
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
        let node_registry_entry = NodeType::EigenDA;
        let _ = find_latest_avs_version(&pool, &node_registry_entry).await?;
        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    fn test_ava_version_parsing(pool: PgPool) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        let node_registry_entry = NodeType::AvaProtocol;
        let _ = find_latest_avs_version(&pool, &node_registry_entry).await?;
        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    fn test_k3labs_version_parsing(pool: PgPool) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        let node_registry_entry = NodeType::K3LabsAvs;
        let _ = find_latest_avs_version(&pool, &node_registry_entry).await?;
        Ok(())
    }

    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    fn test_lagrange_zk_holesky_version_parsing(
        pool: PgPool,
    ) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        let node_registry_entry = NodeType::LagrangeZkWorkerHolesky;
        let _ = find_latest_avs_version(&pool, &node_registry_entry).await?;
        Ok(())
    }
}
