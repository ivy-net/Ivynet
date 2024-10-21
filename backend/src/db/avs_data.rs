use ivynet_core::avs::names::{AvsName, AvsParseError};
use semver::Version;
use serde::{Deserialize, Serialize};
use sqlx::query;
use utoipa::ToSchema;

use crate::error::BackendError;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AvsData {
    pub avs_name: AvsName,
    pub avs_version: Version,
}

#[derive(Clone, Debug)]
pub struct DbAvsData {
    pub avs_name: String,
    pub avs_version: String,
}

impl TryFrom<DbAvsData> for AvsData {
    type Error = AvsParseError;
    fn try_from(db_avs_data: DbAvsData) -> Result<Self, Self::Error> {
        Ok(AvsData {
            avs_name: AvsName::try_from(db_avs_data.avs_name.as_str())?,
            avs_version: Version::parse(&db_avs_data.avs_version)
                .expect("Cannot parse version on dbAvsData"),
        })
    }
}

impl DbAvsData {
    pub async fn get_all_avs_data(pool: &sqlx::PgPool) -> Result<Vec<AvsData>, BackendError> {
        let avs_data: Vec<DbAvsData> =
            sqlx::query_as!(DbAvsData, "SELECT avs_name, avs_version FROM avs_data")
                .fetch_all(pool)
                .await?;

        let avs_data: Vec<AvsData> =
            avs_data.into_iter().filter_map(|e| AvsData::try_from(e).ok()).collect();
        Ok(avs_data)
    }

    pub async fn get_avs_data(
        pool: &sqlx::PgPool,
        avs_name: &AvsName,
    ) -> Result<Option<AvsData>, BackendError> {
        let avs_data: Option<DbAvsData> = sqlx::query_as!(
            DbAvsData,
            "SELECT avs_name, avs_version FROM avs_data WHERE avs_name = $1",
            avs_name.as_str()
        )
        .fetch_optional(pool)
        .await?;

        match avs_data {
            Some(avs_data) => Ok(AvsData::try_from(avs_data).ok()),
            None => Ok(None),
        }
    }

    pub async fn insert_avs_data(
        pool: &sqlx::PgPool,
        avs_data: &AvsData,
    ) -> Result<(), BackendError> {
        query!(
            "INSERT INTO avs_data (avs_name, avs_version) VALUES ($1, $2)",
            avs_data.avs_name.as_str(),
            avs_data.avs_version.to_string()
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete_avs_data(
        pool: &sqlx::PgPool,
        avs_name: &AvsName,
    ) -> Result<(), BackendError> {
        query!("DELETE FROM avs_data WHERE avs_name = $1", avs_name.as_str()).execute(pool).await?;

        Ok(())
    }

    pub async fn set_avs_version(
        pool: &sqlx::PgPool,
        avs_name: &AvsName,
        avs_version: &Version,
    ) -> Result<(), BackendError> {
        query!(
            "INSERT INTO avs_data (avs_name, avs_version) VALUES ($1, $2)
            ON CONFLICT (avs_name)
            DO UPDATE SET avs_version = $2",
            avs_name.as_str(),
            avs_version.to_string(),
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
