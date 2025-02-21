use crate::{error::DatabaseError, AvsVersionHash};
use ivynet_error::ethers::types::Chain;
use ivynet_node_type::NodeType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionType {
    SemVer,
    /// For node types with fixed docker versioning tags, such as `latest` or `holesky`
    FixedVer,
    /// Hybrid version type, for node types with both fixed and semver versioning. Currently used
    /// when a node type has both fixed and semver versioning, and the most reliable way to report
    /// the latest version is to find the semver tag corresponding to the latest tag.
    HybridVer,
    /// Node types that you only build locally
    LocalOnly,
    /// Node types that are opt-in only
    OptInOnly,
}

// TODO: This is really messy, should probably live in core but has a ToSchema dep
impl From<&NodeType> for VersionType {
    fn from(node_type: &NodeType) -> Self {
        match node_type {
            NodeType::Tanssi => VersionType::FixedVer,
            NodeType::Bolt(_) => VersionType::SemVer,
            NodeType::Zellular => VersionType::FixedVer,
            NodeType::AtlasNetwork => VersionType::FixedVer,
            NodeType::Primus => VersionType::SemVer,
            NodeType::DittoNetwork(_) => VersionType::SemVer,
            NodeType::Gasp => VersionType::FixedVer,
            NodeType::EigenDA => VersionType::SemVer,
            NodeType::LagrangeZkWorker => VersionType::SemVer,
            NodeType::LagrangeZKProver => VersionType::FixedVer,
            NodeType::AvaProtocol => VersionType::SemVer,
            NodeType::EOracle => VersionType::HybridVer,
            NodeType::K3LabsAvs => VersionType::FixedVer,
            NodeType::K3LabsAvsHolesky => VersionType::FixedVer,
            NodeType::Predicate => VersionType::SemVer,
            NodeType::Hyperlane(_) => VersionType::SemVer,
            NodeType::WitnessChain => VersionType::SemVer,
            NodeType::Unknown => VersionType::SemVer,
            NodeType::LagrangeStateCommittee => VersionType::SemVer,
            NodeType::Altlayer(_) => VersionType::SemVer,
            NodeType::AltlayerMach(_) => VersionType::SemVer,
            NodeType::Omni => VersionType::FixedVer,
            NodeType::Automata => VersionType::SemVer,
            NodeType::OpenLayerHolesky => VersionType::FixedVer,
            NodeType::OpenLayerMainnet => VersionType::FixedVer,
            NodeType::ChainbaseNetworkV1 => VersionType::SemVer,
            NodeType::ChainbaseNetwork => VersionType::SemVer,
            NodeType::UngateInfiniRoute(_) => VersionType::FixedVer,
            NodeType::AethosHolesky => VersionType::SemVer,
            NodeType::ArpaNetworkNodeClient => VersionType::FixedVer,
            NodeType::Brevis => VersionType::LocalOnly,
            NodeType::PrimevMevCommit(_) => VersionType::LocalOnly,
            NodeType::PrimevBidder => VersionType::LocalOnly,
            NodeType::Nuffle => VersionType::LocalOnly,
            NodeType::AlignedLayer => VersionType::LocalOnly,
            NodeType::GoPlusAVS => VersionType::LocalOnly,
            NodeType::SkateChain(_) => VersionType::LocalOnly,
            NodeType::Blockless => VersionType::LocalOnly,
            NodeType::Redstone => VersionType::LocalOnly,
            NodeType::MishtiNetwork(_) => VersionType::LocalOnly,
            NodeType::Cycle => VersionType::LocalOnly,
            NodeType::Kalypso => VersionType::LocalOnly,
            NodeType::UnifiAVS => VersionType::OptInOnly,
            NodeType::RouterXtendNetwork => VersionType::OptInOnly,
            NodeType::CapxCloud => VersionType::OptInOnly,
            NodeType::Symbiosis => VersionType::OptInOnly,
            NodeType::Radius => VersionType::OptInOnly,
            NodeType::IBTCNetwork => VersionType::OptInOnly,
            NodeType::ZKLink => VersionType::OptInOnly,
            NodeType::HyveDA => VersionType::OptInOnly,
        }
    }
}

impl VersionType {
    pub fn fixed_name(node_type: &NodeType, chain: &Chain) -> Option<&'static str> {
        match (node_type, chain) {
            (NodeType::LagrangeZKProver, _) => Some("latest"),
            (NodeType::Gasp, _) => Some("latest"),
            (NodeType::K3LabsAvs, _) => Some("latest"),
            (NodeType::K3LabsAvsHolesky, _) => Some("latest"),
            (NodeType::EOracle, _) => Some("latest"),
            (NodeType::Omni, _) => Some("main"),
            (NodeType::OpenLayerMainnet, _) => Some("latest"),
            (NodeType::OpenLayerHolesky, _) => Some("latest"),
            (NodeType::ArpaNetworkNodeClient, _) => Some("latest"),
            (NodeType::AtlasNetwork, _) => Some("testnet-eigenlayer"),
            (NodeType::Zellular, _) => Some("latest"),
            (NodeType::Tanssi, _) => Some("latest"),
            _ => None,
        }
    }
}

/// Pulls tags from the db, selects versioning type depending on the node type, and returns the
/// latest version via `(tag, digest).`
pub async fn find_latest_avs_version(
    pool: &sqlx::PgPool,
    node_type: &NodeType,
    chain: &Chain,
) -> Result<(String, String), DatabaseError> {
    let avs_name = node_type.to_string();

    let (tag, digest) = match VersionType::from(node_type) {
        VersionType::FixedVer => {
            let tag = VersionType::fixed_name(node_type, chain)
                .expect("FixedVer should have a fixed name like latest")
                .to_string();
            let digest = AvsVersionHash::get_digest_for_version(pool, &avs_name, &tag).await?;
            (tag, digest)
        }
        VersionType::SemVer => {
            // get all tags from db
            let version_list = AvsVersionHash::get_all_for_type(pool, &avs_name).await?;

            // If a version type is semver, we sanitize the list, discarding the other
            // elements.
            let mut version_vec = version_list
                .iter()
                .filter_map(|version_data| {
                    let raw_tag = version_data.version.clone();
                    let digest = version_data.hash.clone();
                    // sanitize the tag via regex
                    let semver_tag = extract_semver(&raw_tag)?;
                    Some((semver_tag, raw_tag, digest))
                })
                .collect::<Vec<_>>();

            if *chain == Chain::Mainnet {
                // filter prerelease versions
                version_vec = version_vec
                    .into_iter()
                    .filter(|(v, _, _)| v.pre.is_empty())
                    .collect::<Vec<_>>();
            }

            let latest = version_vec
                .iter()
                .max_by_key(|(v, _, _)| v)
                .ok_or(DatabaseError::NoVersionsFound)?;
            (latest.1.to_string(), latest.2.to_string())
        }
        VersionType::HybridVer => {
            let tag = VersionType::fixed_name(node_type, chain).unwrap().to_string();
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
        VersionType::LocalOnly => ("Local".to_string(), "Local_Builds_Only".to_string()),
        VersionType::OptInOnly => ("OptInOnly".to_string(), "OptInOnly".to_string()),
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
    #[sqlx::test(migrations = "../migrations", fixtures("../../fixtures/avs_version_hashes.sql"))]
    async fn test_eigenda_version_parsing(
        pool: PgPool,
    ) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        std::env::set_var("DATABASE_URL", "postgresql://ivy:secret_ivy@localhost:5432/ivynet");
        println!("{:#?}", pool.options());
        let node_registry_entry = NodeType::EigenDA;
        let version = find_latest_avs_version(&pool, &node_registry_entry, &Chain::Mainnet).await?;
        println!("{:?}", version);
        Ok(())
    }

    #[ignore]
    #[sqlx::test(migrations = "../migrations", fixtures("../../fixtures/avs_version_hashes.sql"))]
    async fn test_ava_version_parsing(
        pool: PgPool,
    ) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        std::env::set_var("DATABASE_URL", "postgresql://ivy:secret_ivy@localhost:5432/ivynet");
        let node_registry_entry = NodeType::AvaProtocol;
        let version = find_latest_avs_version(&pool, &node_registry_entry, &Chain::Mainnet).await?;
        println!("{:?}", version);
        Ok(())
    }

    #[ignore]
    #[sqlx::test(migrations = "../migrations", fixtures("../../fixtures/avs_version_hashes.sql"))]
    async fn test_k3labs_version_parsing(
        pool: PgPool,
    ) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        std::env::set_var("DATABASE_URL", "postgresql://ivy:secret_ivy@localhost:5432/ivynet");
        let node_registry_entry = NodeType::K3LabsAvs;
        let version = find_latest_avs_version(&pool, &node_registry_entry, &Chain::Mainnet).await?;
        println!("{:?}", version);
        Ok(())
    }

    #[ignore]
    #[sqlx::test(migrations = "../migrations", fixtures("../../fixtures/avs_version_hashes.sql"))]
    async fn test_lagrange_zk_holesky_version_parsing(
        pool: PgPool,
    ) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        std::env::set_var("DATABASE_URL", "postgresql://ivy:secret_ivy@localhost:5432/ivynet");
        let node_registry_entry = NodeType::LagrangeZkWorker;
        let version = find_latest_avs_version(&pool, &node_registry_entry, &Chain::Holesky).await?;
        println!("{:?}", version);
        Ok(())
    }

    #[ignore]
    #[sqlx::test(migrations = "../migrations", fixtures("../../fixtures/avs_version_hashes.sql"))]
    async fn test_eoracle_version_parsing(
        pool: PgPool,
    ) -> sqlx::Result<(), Box<dyn std::error::Error>> {
        std::env::set_var("DATABASE_URL", "postgresql://ivy:secret_ivy@localhost:5432/ivynet");
        let node_registry_entry = NodeType::EOracle;
        let version = find_latest_avs_version(&pool, &node_registry_entry, &Chain::Mainnet).await?;
        println!("{:?}", version);
        Ok(())
    }
}
