use chrono::NaiveDateTime;
use ivynet_core::{avs::names::AvsName, ethers::types::Chain, utils::try_parse_chain};
use semver::Version;
use serde::{Deserialize, Serialize};
use sqlx::query;
use utoipa::ToSchema;

use crate::error::BackendError;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AvsVersionData {
    pub id: i32,
    pub avs_name: AvsName,
    pub chain: Chain,
    pub latest_version: Version,
    pub breaking_change_version: Option<Version>,
    pub breaking_change_datetime: Option<NaiveDateTime>,
}

#[derive(Clone, Debug)]
pub struct DbAvsVersionData {
    pub id: i32,
    pub avs_name: String,
    pub chain: String,
    pub latest_version: String,
    pub breaking_change_version: Option<String>,
    pub breaking_change_datetime: Option<NaiveDateTime>,
}

impl TryFrom<DbAvsVersionData> for AvsVersionData {
    type Error = BackendError;
    fn try_from(db_avs_version_data: DbAvsVersionData) -> Result<Self, BackendError> {
        Ok(AvsVersionData {
            id: db_avs_version_data.id,
            avs_name: AvsName::try_from(db_avs_version_data.avs_name.as_str())
                .expect("Could not parse AvsName"),
            latest_version: Version::parse(&db_avs_version_data.latest_version)
                .expect("Cannot parse version on dbAvsVersionData"),
            chain: try_parse_chain(&db_avs_version_data.chain).expect("Cannot parse chain"),
            breaking_change_version: db_avs_version_data
                .breaking_change_version
                .and_then(|v| Version::parse(&v).ok()),
            breaking_change_datetime: db_avs_version_data.breaking_change_datetime,
        })
    }
}

impl DbAvsVersionData {
    pub async fn get_all_avs_version(
        pool: &sqlx::PgPool,
    ) -> Result<Vec<AvsVersionData>, BackendError> {
        let avs_version_data: Vec<DbAvsVersionData> =
            sqlx::query_as!(DbAvsVersionData, "SELECT * FROM avs_version_data")
                .fetch_all(pool)
                .await?;

        Ok(avs_version_data
            .into_iter()
            .filter_map(|db_version_data| AvsVersionData::try_from(db_version_data).ok())
            .collect())
    }

    pub async fn get_avs_version(
        pool: &sqlx::PgPool,
        avs_name: &AvsName,
    ) -> Result<Option<AvsVersionData>, BackendError> {
        let db_avs_version_data: Option<DbAvsVersionData> = sqlx::query_as!(
            DbAvsVersionData,
            "SELECT * FROM avs_version_data WHERE avs_name = $1",
            avs_name.as_str()
        )
        .fetch_optional(pool)
        .await?;

        Ok(db_avs_version_data.and_then(|data| AvsVersionData::try_from(data).ok()))
    }

    pub async fn insert_avs_version(
        pool: &sqlx::PgPool,
        avs_version_data: &AvsVersionData,
    ) -> Result<(), BackendError> {
        match (
            &avs_version_data.breaking_change_version,
            &avs_version_data.breaking_change_datetime,
        ) {
            (Some(breaking_change_version), Some(breaking_change_datetime)) => {
                query!(
                    "INSERT INTO avs_version_data (avs_name, latest_version, chain, breaking_change_version, breaking_change_datetime)
                    VALUES ($1, $2, $3, $4, $5)",
                    avs_version_data.avs_name.as_str(),
                    avs_version_data.latest_version.to_string(),
                    avs_version_data.chain.to_string(),
                    breaking_change_version.to_string(),
                    breaking_change_datetime,
                )
                .execute(pool)
                .await?;
            }
            _ => {
                query!(
                    "INSERT INTO avs_version_data (avs_name, latest_version, chain)
                    VALUES ($1, $2, $3)",
                    avs_version_data.avs_name.as_str(),
                    avs_version_data.latest_version.to_string(),
                    avs_version_data.chain.to_string(),
                )
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn delete_avs_version_data(
        pool: &sqlx::PgPool,
        avs_name: &AvsName,
        chain: &Chain,
    ) -> Result<(), BackendError> {
        query!(
            "DELETE FROM avs_version_data WHERE avs_name = $1 AND chain = $2",
            avs_name.as_str(),
            chain.to_string(),
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn set_avs_version(
        pool: &sqlx::PgPool,
        avs_name: &AvsName,
        chain: &Chain,
        latest_version: &Version,
    ) -> Result<(), BackendError> {
        query!(
            "INSERT INTO avs_version_data (avs_name, latest_version, chain)
            VALUES ($1, $2, $3)
            ON CONFLICT (avs_name, chain) DO UPDATE SET latest_version = $2",
            avs_name.as_str(),
            latest_version.to_string(),
            chain.to_string(),
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn set_breaking_change_version(
        pool: &sqlx::PgPool,
        avs_name: &AvsName,
        chain: &Chain,
        breaking_change_version: &Version,
        breaking_change_datetime: &NaiveDateTime,
    ) -> Result<(), BackendError> {
        query!(
            "UPDATE avs_version_data
            SET breaking_change_version = $3, breaking_change_datetime = $4
            WHERE avs_name = $1 AND chain = $2",
            avs_name.as_str(),
            chain.to_string(),
            Some(breaking_change_version.to_string()),
            Some(breaking_change_datetime)
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
