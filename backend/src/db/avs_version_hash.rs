use chrono::NaiveDateTime;

use crate::error::BackendError;

#[derive(Clone, Debug)]
pub struct AvsVersionHash {
    pub id: i64,
    pub avs_type: String,
    pub hash: String,
    pub version: String,
    pub created_at: Option<NaiveDateTime>,
}

impl AvsVersionHash {
    pub async fn get_version(pool: &sqlx::PgPool, hash: &str) -> Result<String, BackendError> {
        let avs_version: AvsVersionHash =
            sqlx::query_as!(AvsVersionHash, "SELECT * FROM avs_version_hash WHERE hash = $1", hash)
                .fetch_one(pool)
                .await?;

        Ok(avs_version.version)
    }

    pub async fn get_digest_for_version(
        pool: &sqlx::PgPool,
        avs_type: &str,
        version: &str,
    ) -> Result<String, BackendError> {
        let avs_version: AvsVersionHash = sqlx::query_as!(
            AvsVersionHash,
            "SELECT * FROM avs_version_hash WHERE avs_type = $1 AND version = $2",
            avs_type,
            version
        )
        .fetch_one(pool)
        .await?;

        Ok(avs_version.hash)
    }

    pub async fn get_all_for_type(
        pool: &sqlx::PgPool,
        avs_type: &str,
    ) -> Result<Vec<AvsVersionHash>, BackendError> {
        let tags = sqlx::query_as!(
            AvsVersionHash,
            r#"SELECT * FROM avs_version_hash WHERE avs_type = $1"#,
            avs_type
        )
        .fetch_all(pool)
        .await?;

        Ok(tags)
    }

    pub async fn add_version(
        pool: &sqlx::PgPool,
        avs_type: &str,
        hash: &str,
        version: &str,
    ) -> Result<(), BackendError> {
        sqlx::query!(
            "INSERT INTO avs_version_hash (avs_type, hash, version) values ($1, $2, $3)",
            avs_type,
            hash,
            version
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_version(
        pool: &sqlx::PgPool,
        avs_type: &str,
        hash: &str,
        version: &str,
    ) -> Result<(), BackendError> {
        // Uses above for reference but runs an insert and updates on conflict
        sqlx::query!(
            "INSERT INTO avs_version_hash (avs_type, hash, version) values ($1, $2, $3) ON CONFLICT (avs_type, version) DO UPDATE SET version = $3, hash = $2",
            avs_type,
            hash,
            version
        ).execute(pool).await?;

        Ok(())
    }
}
