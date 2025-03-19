use ivynet_notifications::{OrganizationDatabase, RegistrationResult, UnregistrationResult};
use std::collections::HashSet;

use sqlx::PgPool;

use crate::{Account, NotificationSettings, ServiceSettings};

/// Backend implementation for alert database operations
#[derive(Debug, Clone)]
struct AlertDbBackend {
    pool: PgPool,
}

impl AlertDbBackend {
    fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Adds a chat to an organization's notification settings
    ///
    /// Returns RegistrationResult indicating the outcome of the registration attempt
    pub async fn add_chat(&self, email: &str, password: &str, chat_id: &str) -> RegistrationResult {
        tracing::debug!("adding chat to organization: {}", chat_id);
        match Account::verify(&self.pool, email, password).await {
            Ok(account) => {
                let result = NotificationSettings::add_chat(
                    &self.pool,
                    account.organization_id.try_into().unwrap_or(0),
                    chat_id,
                )
                .await;
                tracing::debug!("result: {:?}", result);
                match result {
                    Ok(r) => match r {
                        Some(_) => RegistrationResult::Success,
                        None => RegistrationResult::AlreadyRegistered,
                    },
                    Err(e) => RegistrationResult::DatabaseError(e.to_string()),
                }
            }
            Err(e) => {
                tracing::error!("Failed to verify account: {}", e);
                RegistrationResult::AuthenticationFailed
            }
        }
    }

    /// Removes a chat from the notification settings
    ///
    /// Returns true if successful, false otherwise
    /// WARN: Ensure this is only callable by either the telegram bot or via a verified account on
    /// the backend.
    pub async fn remove_chat(&self, chat_id: &str) -> UnregistrationResult {
        tracing::debug!("removing chat from database: {}", chat_id);
        let result = ServiceSettings::remove_chat(&self.pool, chat_id).await;
        match result {
            Ok(rows_affected) => {
                if rows_affected > 0 {
                    UnregistrationResult::Success
                } else {
                    UnregistrationResult::ChatNotRegistered
                }
            }
            Err(e) => {
                tracing::error!("Failed to remove chat: {}", e);
                UnregistrationResult::DatabaseError(e.to_string())
            }
        }
    }

    /// Gets all chat IDs for an organization
    ///
    /// Returns an empty HashSet if there's an error
    pub async fn chats_for(&self, organization_id: u64) -> HashSet<String> {
        match NotificationSettings::get(&self.pool, organization_id).await {
            Ok(settings) => settings.telegram_chats,
            Err(e) => {
                tracing::error!("Failed to get chats for organization {}: {}", organization_id, e);
                HashSet::new()
            }
        }
    }

    /// Gets all email addresses for an organization
    ///
    /// Returns an empty HashSet if there's an error
    pub async fn emails(&self, organization_id: u64) -> HashSet<String> {
        match NotificationSettings::get(&self.pool, organization_id).await {
            Ok(settings) => settings.sendgrid_emails,
            Err(e) => {
                tracing::error!("Failed to get emails for organization {}: {}", organization_id, e);
                HashSet::new()
            }
        }
    }

    /// Gets the PagerDuty integration key for an organization
    ///
    /// Returns None if there's an error or no key is set
    pub async fn integration_key(&self, organization_id: u64) -> HashSet<String> {
        match NotificationSettings::get(&self.pool, organization_id).await {
            Ok(settings) => settings.pagerduty_keys,
            Err(e) => {
                tracing::error!(
                    "Failed to get PagerDuty integration keys for organization {}: {}",
                    organization_id,
                    e
                );
                HashSet::new()
            }
        }
    }
}

/// Database interface for alert-related operations
#[derive(Clone, Debug)]
pub struct AlertDb(AlertDbBackend);

impl AlertDb {
    pub fn new(pool: PgPool) -> Self {
        Self(AlertDbBackend::new(pool))
    }
}

#[ivynet_grpc::async_trait]
impl OrganizationDatabase for AlertDb {
    async fn register_chat(
        &self,
        chat_id: &str,
        email: &str,
        password: &str,
    ) -> RegistrationResult {
        let db = &self.0;
        db.add_chat(email, password, chat_id).await
    }

    async fn unregister_chat(&self, chat_id: &str) -> UnregistrationResult {
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

    async fn get_pd_integration_keys_for_organization(
        &self,
        organization_id: u64,
    ) -> HashSet<String> {
        let db = &self.0;
        db.integration_key(organization_id).await
    }
}
