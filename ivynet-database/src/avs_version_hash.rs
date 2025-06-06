use chrono::NaiveDateTime;
use ivynet_node_type::NodeType;

use crate::{data::avs_version::extract_semver, error::DatabaseError};

#[derive(Clone, Debug)]
pub struct AvsVersionHash {
    pub avs_type: NodeType,
    pub hash: String,
    pub version: String,
    pub created_at: Option<NaiveDateTime>,
}

#[derive(Clone, Debug)]
pub struct DbAvsVersionHash {
    pub id: i64,
    pub avs_type: String,
    pub hash: String,
    pub version: String,
    pub created_at: Option<NaiveDateTime>,
}

impl From<DbAvsVersionHash> for AvsVersionHash {
    fn from(db_avs_version: DbAvsVersionHash) -> Self {
        AvsVersionHash {
            avs_type: NodeType::from(db_avs_version.avs_type.as_str()),
            hash: db_avs_version.hash,
            version: db_avs_version.version,
            created_at: db_avs_version.created_at,
        }
    }
}

impl AvsVersionHash {
    pub async fn get_version(pool: &sqlx::PgPool, hash: &str) -> Result<String, DatabaseError> {
        let avs_versions: Vec<DbAvsVersionHash> = sqlx::query_as!(
            DbAvsVersionHash,
            "SELECT * FROM avs_version_hash WHERE hash = $1",
            hash
        )
        .fetch_all(pool)
        .await?;

        let semver_tags = avs_versions
            .clone()
            .into_iter()
            .filter_map(|version| {
                let semver_tag = extract_semver(&version.version)?;
                Some((semver_tag, version))
            })
            .collect::<Vec<_>>();
        let latest = semver_tags.iter().max_by_key(|(v, _)| v);

        if let Some(latest) = latest {
            return Ok(latest.1.version.clone());
        }

        if !avs_versions.is_empty() {
            return Ok(avs_versions[0].version.clone());
        }

        Err(DatabaseError::NoVersionsFound)
    }

    pub async fn get_versions_from_digest(
        pool: &sqlx::PgPool,
        avs_type: &str,
        digest: &str,
    ) -> Result<Vec<String>, DatabaseError> {
        let tags = sqlx::query_as!(
            DbAvsVersionHash,
            r#"SELECT * FROM avs_version_hash WHERE avs_type = $1 AND hash = $2"#,
            avs_type,
            digest
        )
        .fetch_all(pool)
        .await?;

        Ok(tags.into_iter().map(|t| t.version).collect())
    }

    pub async fn get_avs_type_from_hash(
        pool: &sqlx::PgPool,
        hash: &str,
    ) -> Result<NodeType, DatabaseError> {
        let avs_version: DbAvsVersionHash = sqlx::query_as!(
            DbAvsVersionHash,
            "SELECT * FROM avs_version_hash WHERE hash = $1",
            hash
        )
        .fetch_one(pool)
        .await?;

        Ok(NodeType::from(avs_version.avs_type.as_str()))
    }

    pub async fn get_digest_for_version(
        pool: &sqlx::PgPool,
        avs_type: &str,
        version: &str,
    ) -> Result<String, DatabaseError> {
        let avs_version: DbAvsVersionHash = sqlx::query_as!(
            DbAvsVersionHash,
            "SELECT * FROM avs_version_hash WHERE avs_type = $1 AND version = $2",
            avs_type,
            version
        )
        .fetch_one(pool)
        .await?;

        Ok(avs_version.hash)
    }

    pub async fn get_versions_from_digests(
        pool: &sqlx::PgPool,
        digests: &[String],
    ) -> Result<Vec<(String, String)>, DatabaseError> {
        let avs_versions = sqlx::query_as!(
            DbAvsVersionHash,
            "SELECT * FROM avs_version_hash WHERE hash = ANY($1)",
            digests,
        )
        .fetch_all(pool)
        .await?;
        Ok(avs_versions.into_iter().map(|v| (v.hash, v.avs_type)).collect::<Vec<_>>())
    }

    pub async fn get_all_for_type(
        pool: &sqlx::PgPool,
        avs_type: &str,
    ) -> Result<Vec<AvsVersionHash>, DatabaseError> {
        let tags = sqlx::query_as!(
            DbAvsVersionHash,
            r#"SELECT * FROM avs_version_hash WHERE avs_type = $1"#,
            avs_type
        )
        .fetch_all(pool)
        .await?;

        Ok(tags.into_iter().map(|t| t.into()).collect())
    }

    pub async fn add_version(
        pool: &sqlx::PgPool,
        avs_type: &NodeType,
        hash: &str,
        version: &str,
    ) -> Result<(), DatabaseError> {
        sqlx::query!(
            "INSERT INTO avs_version_hash (avs_type, hash, version) values ($1, $2, $3)",
            avs_type.to_string(),
            hash,
            version
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_version(
        pool: &sqlx::PgPool,
        avs_type: &NodeType,
        hash: &str,
        version: &str,
    ) -> Result<(), DatabaseError> {
        // Uses above for reference but runs an insert and updates on conflict
        sqlx::query!(
            "INSERT INTO avs_version_hash (avs_type, hash, version) values ($1, $2, $3) ON CONFLICT (avs_type, version) DO UPDATE SET version = $3, hash = $2",
            avs_type.to_string(),
            hash,
            version
        ).execute(pool).await?;

        Ok(())
    }

    pub async fn delete_avses_from_avs_version_hash(
        pool: &sqlx::PgPool,
        avs_types_to_keep: &[NodeType],
    ) -> Result<(), DatabaseError> {
        // First get all unique node types from DB
        let all_types =
            sqlx::query!("SELECT DISTINCT avs_type FROM avs_version_hash").fetch_all(pool).await?;

        // Convert keep list to strings for comparison
        let keep_set: Vec<String> = avs_types_to_keep.iter().map(|t| t.to_string()).collect();

        // Find types to delete
        let types_to_delete: Vec<String> = all_types
            .into_iter()
            .map(|row| row.avs_type)
            .filter(|node_type| !keep_set.contains(node_type))
            .collect();

        // If we found types to delete, remove them
        if !types_to_delete.is_empty() {
            let result = sqlx::query!(
                "DELETE FROM avs_version_hash WHERE avs_type = ANY($1)",
                &types_to_delete
            )
            .execute(pool)
            .await?;

            println!(
                "Deleted {} rows for node types: {:?}",
                result.rows_affected(),
                types_to_delete
            );
        }
        Ok(())
    }
}
