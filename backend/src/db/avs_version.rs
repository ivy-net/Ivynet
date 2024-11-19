use std::collections::HashMap;

use chrono::NaiveDateTime;
use ivynet_core::{ethers::types::Chain, node_type::NodeType, utils::try_parse_chain};
use semver::Version;
use serde::{Deserialize, Serialize};
use sqlx::query;
use utoipa::ToSchema;

use crate::error::BackendError;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AvsVersionData {
    #[serde(flatten)]
    pub id: AvsID,
    #[serde(flatten)]
    pub vd: VersionData,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct VersionData {
    pub latest_version: Version,
    pub breaking_change_version: Option<Version>,
    pub breaking_change_datetime: Option<NaiveDateTime>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, Eq, PartialEq, Hash)]
pub struct AvsID {
    pub avs_name: NodeType,
    pub chain: Chain,
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
        let version_data = {
            VersionData {
                latest_version: Version::parse(&db_avs_version_data.latest_version)
                    .expect("Cannot parse version on dbAvsVersionData"),
                breaking_change_version: db_avs_version_data
                    .breaking_change_version
                    .and_then(|v| Version::parse(&v).ok()),
                breaking_change_datetime: db_avs_version_data.breaking_change_datetime,
            }
        };
        let id = AvsID {
            avs_name: NodeType::from(db_avs_version_data.avs_name.as_str()),
            chain: try_parse_chain(&db_avs_version_data.chain).expect("Cannot parse chain"),
        };
        Ok(AvsVersionData { id, vd: version_data })
    }
}

impl DbAvsVersionData {
    pub async fn get_all_avs_version(
        pool: &sqlx::PgPool,
    ) -> Result<HashMap<AvsID, VersionData>, BackendError> {
        let avs_version_data: Vec<DbAvsVersionData> =
            sqlx::query_as!(DbAvsVersionData, "SELECT * FROM avs_version_data")
                .fetch_all(pool)
                .await?;

        Ok(avs_version_data
            .into_iter()
            .filter_map(|data| AvsVersionData::try_from(data).ok().map(|data| (data.id, data.vd)))
            .collect())
    }

    pub async fn get_avs_version(
        pool: &sqlx::PgPool,
        avs_name: &NodeType,
    ) -> Result<Vec<AvsVersionData>, BackendError> {
        let db_avs_version_data: Vec<DbAvsVersionData> = sqlx::query_as!(
            DbAvsVersionData,
            "SELECT * FROM avs_version_data WHERE avs_name = $1",
            avs_name.to_string()
        )
        .fetch_all(pool)
        .await?;

        let data: Vec<AvsVersionData> = db_avs_version_data
            .into_iter()
            .filter_map(|data| AvsVersionData::try_from(data).ok())
            .collect();

        Ok(data)
    }

    pub async fn get_avs_version_with_chain(
        pool: &sqlx::PgPool,
        avs_name: &NodeType,
        chain: &Chain,
    ) -> Result<Option<AvsVersionData>, BackendError> {
        let db_avs_version_data: Option<DbAvsVersionData> = sqlx::query_as!(
            DbAvsVersionData,
            "SELECT * FROM avs_version_data WHERE avs_name = $1 AND chain = $2",
            avs_name.to_string(),
            chain.to_string(),
        )
        .fetch_optional(pool)
        .await?;

        match db_avs_version_data {
            Some(data) => Ok(Some(AvsVersionData::try_from(data)?)),
            None => Ok(None),
        }
    }

    pub async fn insert_avs_version(
        pool: &sqlx::PgPool,
        avs_version_data: &AvsVersionData,
    ) -> Result<(), BackendError> {
        match (
            &avs_version_data.vd.breaking_change_version,
            &avs_version_data.vd.breaking_change_datetime,
        ) {
            (Some(breaking_change_version), Some(breaking_change_datetime)) => {
                query!(
                    "INSERT INTO avs_version_data (avs_name, latest_version, chain, breaking_change_version, breaking_change_datetime)
                    VALUES ($1, $2, $3, $4, $5)",
                    avs_version_data.id.avs_name.to_string(),
                    avs_version_data.vd.latest_version.to_string(),
                    avs_version_data.id.chain.to_string(),
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
                    avs_version_data.id.avs_name.to_string(),
                    avs_version_data.vd.latest_version.to_string(),
                    avs_version_data.id.chain.to_string(),
                )
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn delete_avs_version_data(
        pool: &sqlx::PgPool,
        avs_name: &NodeType,
        chain: &Chain,
    ) -> Result<(), BackendError> {
        query!(
            "DELETE FROM avs_version_data WHERE avs_name = $1 AND chain = $2",
            avs_name.to_string(),
            chain.to_string(),
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn set_avs_version(
        pool: &sqlx::PgPool,
        avs_name: &NodeType,
        chain: &Chain,
        latest_version: &Version,
    ) -> Result<(), BackendError> {
        query!(
            "INSERT INTO avs_version_data (avs_name, latest_version, chain)
            VALUES ($1, $2, $3)
            ON CONFLICT (avs_name, chain) DO UPDATE SET latest_version = $2",
            avs_name.to_string(),
            latest_version.to_string(),
            chain.to_string(),
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn set_breaking_change_version(
        pool: &sqlx::PgPool,
        avs_name: &NodeType,
        chain: &Chain,
        breaking_change_version: &Version,
        breaking_change_datetime: &NaiveDateTime,
    ) -> Result<(), BackendError> {
        query!(
            "UPDATE avs_version_data
            SET breaking_change_version = $3, breaking_change_datetime = $4
            WHERE avs_name = $1 AND chain = $2",
            avs_name.to_string(),
            chain.to_string(),
            Some(breaking_change_version.to_string()),
            Some(breaking_change_datetime)
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
