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

    pub async fn get_all_tags_for_type(
        pool: &sqlx::PgPool,
        avs_type: &str,
    ) -> Result<Vec<String>, BackendError> {
        println!("Query debug: SELECT * FROM avs_version_hash WHERE avs_type = {}", avs_type);
        let tags = sqlx::query_as!(
            AvsVersionHash,
            r#"SELECT * FROM avs_version_hash WHERE avs_type = $1"#,
            avs_type
        )
        .fetch_all(pool)
        .await?;

        let tags_test = sqlx::query_as!(
            AvsVersionHash,
            r#"SELECT * FROM avs_version_hash WHERE avs_type = 'eigenda'"#,
        )
        .fetch_all(pool)
        .await?;

        println!("TAGS TEST: {:?}", tags_test.len());

        Ok(tags.into_iter().map(|tag| tag.version).collect())
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
}
