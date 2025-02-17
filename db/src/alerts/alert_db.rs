use ivynet_notifications::OrganizationDatabase;
use std::{collections::HashSet, sync::Arc};
use tokio::sync::Mutex;

use sqlx::PgPool;

use crate::{Account, OrganizationNotifications};

#[derive(Debug)]
struct AlertDbBackend {
    pool: Arc<PgPool>,
}

impl AlertDbBackend {
    fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn add_chat(&mut self, email: &str, password: &str, chat_id: &str) -> bool {
        if let Ok(account) = Account::verify(&self.pool, email, password).await {
            if let Ok(_) = OrganizationNotifications::add_chat(
                &self.pool,
                account.organization_id as u64,
                chat_id,
            )
            .await
            {
                return true;
            }
        }
        false
    }

    pub async fn remove_chat(&mut self, chat_id: &str) -> bool {
        if let Ok(_) = OrganizationNotifications::remove_chat(&self.pool, chat_id).await {
            return true;
        }

        false
    }

    pub async fn chats_for(&self, organization_id: u64) -> HashSet<String> {
        let mut chats: Vec<String> =
            OrganizationNotifications::get_all_chats(&self.pool, organization_id)
                .await
                .unwrap_or_default();

        chats.drain(..).collect()
    }

    pub async fn emails(&self, organization_id: u64) -> HashSet<String> {
        let mut emails: Vec<String> =
            OrganizationNotifications::get_all_emails(&self.pool, organization_id)
                .await
                .unwrap_or_default();

        emails.drain(..).collect()
    }

    pub async fn integration_key(&self, organization_id: u64) -> Option<String> {
        OrganizationNotifications::get_pagerduty_integration(&self.pool, organization_id)
            .await
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub struct AlertDb(Arc<Mutex<AlertDbBackend>>);

impl AlertDb {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self(Arc::new(Mutex::new(AlertDbBackend::new(pool))))
    }
}

#[ivynet_grpc::async_trait]
impl OrganizationDatabase for AlertDb {
    async fn register_chat(&self, chat_id: &str, email: &str, password: &str) -> bool {
        let mut db = self.0.lock().await;
        db.add_chat(email, password, chat_id).await
    }

    async fn unregister_chat(&self, chat_id: &str) -> bool {
        let mut db = self.0.lock().await;
        db.remove_chat(chat_id).await
    }

    async fn get_emails_for_organization(&self, organization_id: u64) -> HashSet<String> {
        let db = self.0.lock().await;
        db.emails(organization_id).await
    }

    async fn get_chats_for_organization(&self, organization_id: u64) -> HashSet<String> {
        let db = self.0.lock().await;
        db.chats_for(organization_id).await
    }

    async fn get_pd_integration_key_for_organization(
        &self,
        organization_id: u64,
    ) -> Option<String> {
        let db = self.0.lock().await;
        db.integration_key(organization_id).await
    }
}
