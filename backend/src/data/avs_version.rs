use ivynet_node_type::NodeType;
use tracing::info;

use crate::{db::AvsVersionHash, error::BackendError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionType {
    SemVer,
    /// For node types with fixed docker versioning tags, such as `latest` or `holesky`
    FixedVer,
    /// Hybrid version type, for node types with both fixed and semver versioning. Currently used
    /// when a node type has both fixed and semver versioning, and the most reliable way to report
    /// the latest version is to find the semver tag corresponding to the latest tag.
    HybridVer,
}

// TODO: This is really messy, should probably live in core but has a ToSchema dep
impl From<&NodeType> for VersionType {
    fn from(node_type: &NodeType) -> Self {
        match node_type {
            NodeType::EigenDA => VersionType::SemVer,
            NodeType::LagrangeZkWorkerHolesky => VersionType::FixedVer,
            NodeType::LagrangeZkWorkerMainnet => VersionType::FixedVer,
            NodeType::AvaProtocol => VersionType::SemVer,
            NodeType::EOracle => VersionType::HybridVer,
            NodeType::K3LabsAvs => VersionType::FixedVer,
            NodeType::K3LabsAvsHolesky => VersionType::FixedVer,
            NodeType::Predicate => VersionType::SemVer,
            NodeType::Hyperlane => VersionType::SemVer,
            NodeType::WitnessChain => VersionType::SemVer,
            NodeType::Unknown => VersionType::SemVer,
            NodeType::LagrangeStateCommittee => VersionType::SemVer,
            NodeType::Altlayer(_any) => VersionType::SemVer,
            NodeType::AltlayerMach(_any) => VersionType::SemVer,
            NodeType::Omni => VersionType::FixedVer,
            NodeType::Automata => VersionType::SemVer,
            NodeType::OpenLayerHolesky => VersionType::FixedVer,
            NodeType::OpenLayerMainnet => VersionType::FixedVer,
            NodeType::ChainbaseNetworkV1 => VersionType::SemVer,
            NodeType::ChainbaseNetwork => VersionType::SemVer,
            NodeType::UngateInfiniRouteBase => VersionType::FixedVer,
            NodeType::UngateInfiniRoutePolygon => VersionType::FixedVer,
            NodeType::AethosHolesky => VersionType::SemVer,
            NodeType::ArpaNetworkNodeClient => VersionType::FixedVer,
            NodeType::Brevis => {
                unreachable!("Brevis has no docker versioning, fix in all_known_with_repo")
            }
            NodeType::PrimevMevCommit => {
                unreachable!("PrimevMevCommit has no docker versioning, fix in all_known_with_repo")
            }
            NodeType::AlignedLayer => {
                unreachable!("AlignedLayer has no docker versioning, fix in all_known_with_repo")
            }
            NodeType::GoPlusAVS => {
                unreachable!("GoPlusAVS has no docker versioning, fix in all_known_with_repo")
            }
            NodeType::SkateChainBase => {
                unreachable!("SkateChainBase has no docker versioning, fix in all_known_with_repo")
            }
            NodeType::SkateChainMantle => {
                unreachable!(
                    "SkateChainMantle has no docker versioning, fix in all_known_with_repo"
                )
            }
            NodeType::UnifiAVS => {
                unreachable!("UnifiAVS has no docker versioning, fix in all_known_with_repo")
            }
        }
    }
}

impl VersionType {
    pub fn fixed_name(node_type: &NodeType) -> Option<&'static str> {
        match node_type {
            NodeType::LagrangeZkWorkerHolesky => Some("holesky"),
            NodeType::LagrangeZkWorkerMainnet => Some("mainnet"),
            NodeType::K3LabsAvs => Some("latest"),
            NodeType::K3LabsAvsHolesky => Some("latest"),
            NodeType::EOracle => Some("latest"),
            NodeType::Omni => Some("latest"),
            NodeType::OpenLayerMainnet => Some("latest"),
            NodeType::OpenLayerHolesky => Some("latest"),
            NodeType::ArpaNetworkNodeClient => Some("latest"),
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

    let (tag, digest) = match VersionType::from(node_type) {
        VersionType::FixedVer => {
            let tag = VersionType::fixed_name(node_type)
                .expect("FixedVer should have a fixed name like latest")
                .to_string();
            let digest = AvsVersionHash::get_digest_for_version(pool, &avs_name, &tag).await?;
            (tag, digest)
        }
        VersionType::SemVer => {
            // get all tags from db
            let version_list = AvsVersionHash::get_all_for_type(pool, &avs_name).await?;
            info!("Found {} tags for {}", version_list.len(), avs_name);

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

            let latest = version_vec
                .iter()
                .max_by_key(|(v, _, _)| v)
                .ok_or(BackendError::NoVersionsFound)?;
            (latest.1.to_string(), latest.2.to_string())
        }
        VersionType::HybridVer => {
            let tag = VersionType::fixed_name(node_type).unwrap().to_string();
            let digest = AvsVersionHash::get_digest_for_version(pool, &avs_name, &tag).await?;
            // Fetch tags and filter out non-semver tags, then sort to find max version of various
            // potential valid tags.
            let vaild_semver_tags =
                AvsVersionHash::get_versions_from_digest(pool, &avs_name, &digest)
                    .await?
                    .into_iter()
                    .filter_map(|version| {
                        let semver_tag = extract_semver(&version)?;
                        Some((semver_tag, version))
                    })
                    .collect::<Vec<_>>();
            let latest = vaild_semver_tags.iter().max_by_key(|(v, _)| v);
            match latest {
                Some(latest) => (latest.1.clone(), digest),
                None => (tag, digest),
            }
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
const SEMVER_REGEX: &str = r#"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?"#;

#[cfg(test)]
mod avs_version_tests {
    use super::*;
    use semver::Version;
    use sqlx::PgPool;

    #[test]
    fn test_semver_regex() {
        let re = regex::Regex::new(SEMVER_REGEX).unwrap();

        let source_and_expected_matches = [
            ("v1.2.3.4", "1.2.3"),
            ("1.2.3.4", "1.2.3"),
            ("1.2.3", "1.2.3"),
            ("agent-1.2.3-other", "1.2.3-other"),
            ("agent-1.2.3.4-other", "1.2.3"),
            ("0.1.2", "0.1.2"),
            ("1.0.0-alpha", "1.0.0-alpha"),
            ("1.0.0-alpha.1", "1.0.0-alpha.1"),
            ("1.0.0-0.3.7", "1.0.0-0.3.7"),
            ("1.0.0-x.7.z.92", "1.0.0-x.7.z.92"),
            ("1.0.0+20130313144700", "1.0.0+20130313144700"),
            ("1.0.0-beta+exp.sha.5114f85", "1.0.0-beta+exp.sha.5114f85"),
        ];

        let only_expected = source_and_expected_matches
            .iter()
            .map(|i| re.find(i.0).map(|m| m.as_str()).unwrap())
            .collect::<Vec<_>>();

        let matches: Vec<_> = source_and_expected_matches
            .iter()
            .map(|i| re.find(i.0).map(|m| m.as_str()).unwrap())
            .collect();

        assert_eq!(matches, only_expected);

        // assert all matches are valid SemVer
        matches.iter().for_each(|m| {
            Version::parse(m).unwrap();
        });
    }

    // TODO: These tests need to be more abstract and run over dummy data instead of live db data.

    #[ignore]
    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    async fn test_eigenda_version_parsing(
        pool: PgPool,
    ) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        std::env::set_var("DATABASE_URL", "postgresql://ivy:secret_ivy@localhost:5432/ivynet");
        println!("{:#?}", pool.options());
        let node_registry_entry = NodeType::EigenDA;
        let version = find_latest_avs_version(&pool, &node_registry_entry).await?;
        println!("{:?}", version);
        Ok(())
    }

    #[ignore]
    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    async fn test_ava_version_parsing(
        pool: PgPool,
    ) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        std::env::set_var("DATABASE_URL", "postgresql://ivy:secret_ivy@localhost:5432/ivynet");
        let node_registry_entry = NodeType::AvaProtocol;
        let version = find_latest_avs_version(&pool, &node_registry_entry).await?;
        println!("{:?}", version);
        Ok(())
    }

    #[ignore]
    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    async fn test_k3labs_version_parsing(
        pool: PgPool,
    ) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        std::env::set_var("DATABASE_URL", "postgresql://ivy:secret_ivy@localhost:5432/ivynet");
        let node_registry_entry = NodeType::K3LabsAvs;
        let version = find_latest_avs_version(&pool, &node_registry_entry).await?;
        println!("{:?}", version);
        Ok(())
    }

    #[ignore]
    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    async fn test_lagrange_zk_holesky_version_parsing(
        pool: PgPool,
    ) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        std::env::set_var("DATABASE_URL", "postgresql://ivy:secret_ivy@localhost:5432/ivynet");
        let node_registry_entry = NodeType::LagrangeZkWorkerHolesky;
        let version = find_latest_avs_version(&pool, &node_registry_entry).await?;
        println!("{:?}", version);
        Ok(())
    }

    #[ignore]
    #[sqlx::test(fixtures("../../fixtures/avs_version_hashes.sql"))]
    async fn test_eoracle_version_parsing(
        pool: PgPool,
    ) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        std::env::set_var("DATABASE_URL", "postgresql://ivy:secret_ivy@localhost:5432/ivynet");
        let node_registry_entry = NodeType::EOracle;
        let version = find_latest_avs_version(&pool, &node_registry_entry).await?;
        println!("{:?}", version);
        Ok(())
    }
}
