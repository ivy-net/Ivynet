use ivynet_notifications::OrganizationDatabase;
use std::collections::HashSet;

use sqlx::PgPool;

use crate::{Account, NotificationSettings};

#[derive(Debug, Clone)]
struct AlertDbBackend {
    pool: PgPool,
}

impl AlertDbBackend {
    fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn add_chat(&self, email: &str, password: &str, chat_id: &str) -> bool {
        if let Ok(account) = Account::verify(&self.pool, email, password).await {
            if NotificationSettings::add_chat(
                &self.pool,
                account.organization_id as u64,
                chat_id,
            )
            .await
            .is_ok()
            {
                return true;
            }
        }
        false
    }

    pub async fn remove_chat(&self, chat_id: &str) -> bool {
        if NotificationSettings::remove_chat(&self.pool, chat_id).await.is_ok() {
            return true;
        }

        false
    }

    pub async fn chats_for(&self, organization_id: u64) -> HashSet<String> {
        let chats: Vec<String> =
            NotificationSettings::get_all_chats(&self.pool, organization_id)
                .await
                .unwrap_or_default();

        chats.into_iter().collect()
    }

    pub async fn emails(&self, organization_id: u64) -> HashSet<String> {
        let emails: Vec<String> =
            NotificationSettings::get_all_emails(&self.pool, organization_id)
                .await
                .unwrap_or_default();

        emails.into_iter().collect()
    }

    pub async fn integration_key(&self, organization_id: u64) -> Option<String> {
        NotificationSettings::get_pagerduty_integration(&self.pool, organization_id)
            .await
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub struct AlertDb(AlertDbBackend);

impl AlertDb {
    pub fn new(pool: PgPool) -> Self {
        Self(AlertDbBackend::new(pool))
    }
}

#[ivynet_grpc::async_trait]
impl OrganizationDatabase for AlertDb {
    async fn register_chat(&self, chat_id: &str, email: &str, password: &str) -> bool {
        let db = &self.0;
        db.add_chat(email, password, chat_id).await
    }

    async fn unregister_chat(&self, chat_id: &str) -> bool {
        let db = &self.0;
        db.remove_chat(chat_id).await
    }

    async fn get_emails_for_organization(&self, organization_id: u64) -> HashSet<String> {
        let db = &self.0;
        db.emails(organization_id).await
    }

    async fn get_chats_for_organization(&self, organization_id: u64) -> HashSet<String> {
        let db = &self.0;
        db.chats_for(organization_id).await
    }

    async fn get_pd_integration_key_for_organization(
        &self,
        organization_id: u64,
    ) -> Option<String> {
        let db = &self.0;
        db.integration_key(organization_id).await
    }
}
