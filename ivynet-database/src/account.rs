use crate::error::DatabaseError;

use chrono::{NaiveDateTime, Utc};
use ivynet_error::ethers::types::Address;
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{avs::Avs, client::Client, machine::Machine, verification::Verification, Organization};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, sqlx::Type, Deserialize, Serialize, ToSchema)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum Role {
    Owner,
    Admin,
    User,
    Reader,
}

impl Role {
    pub fn is_admin(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }

    pub fn can_write(&self) -> bool {
        match self {
            Role::Owner | Role::Admin | Role::User => true,
            Role::Reader => false,
        }
    }
}

#[derive(sqlx::FromRow, Deserialize, Serialize, Clone, Debug)]
pub struct Account {
    pub user_id: i64,
    pub organization_id: i64,
    pub email: String,
    pub password: String,
    pub role: Role,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl Account {
    pub async fn get_all(pool: &PgPool) -> Result<Vec<Account>, DatabaseError> {
        let accounts = sqlx::query_as!(
            Account,
            r#"SELECT user_id, organization_id, email, password, role AS "role!: Role", created_at, updated_at FROM account"#
        )
            .fetch_all(pool)
            .await?;

        Ok(accounts)
    }

    pub async fn get_all_for_organization(
        pool: &PgPool,
        organization_id: i64,
    ) -> Result<Vec<Account>, DatabaseError> {
        let accounts = sqlx::query_as!(
            Account,
            r#"SELECT user_id, organization_id, email, password, role AS "role!: Role", created_at, updated_at FROM account WHERE organization_id = $1"#,
            organization_id
        )
        .fetch_all(pool)
        .await?;

        Ok(accounts)
    }

    pub async fn verify(
        pool: &PgPool,
        email: &str,
        password: &str,
    ) -> Result<Account, DatabaseError> {
        let account = sqlx::query_as!(
            Account,
            r#"SELECT
                a.user_id,
                a.organization_id,
                a.email,
                a.password,
                a.role AS "role!: Role",
                a.created_at,
                a.updated_at
              FROM account a
              JOIN organization o ON a.organization_id = o.organization_id
              WHERE a.email = $1 AND a.password = $2 AND o.verified = TRUE"#,
            email,
            sha256::digest(password)
        )
        .fetch_one(pool)
        .await?;

        Ok(account)
    }

    pub async fn set_password(
        &self,
        pool: &PgPool,
        password: &str,
    ) -> Result<Account, DatabaseError> {
        let account = sqlx::query_as!(
            Account,
            r#"UPDATE account SET password = $1 WHERE email = $2
                    RETURNING user_id, organization_id, email, password, role AS "role: _", created_at, updated_at"#,
            sha256::digest(password),
            self.email,
        )
        .fetch_one(pool)
        .await?;
        Ok(account)
    }

    pub async fn get(pool: &PgPool, id: u64) -> Result<Account, DatabaseError> {
        let account = sqlx::query_as!(
        Account,
        r#"SELECT user_id, organization_id, email, password, role AS "role!: Role", created_at, updated_at FROM account WHERE user_id = $1"#,
        id as i64
    )
    .fetch_one(pool)
    .await?;

        Ok(account)
    }

    pub async fn exists(pool: &PgPool, email: &str) -> Result<bool, DatabaseError> {
        match sqlx::query_as!(
            Account,
            r#"SELECT user_id, organization_id, email, password, role AS "role!: Role", created_at, updated_at FROM account WHERE email = $1"#,
            email)
            .fetch_one(pool)
            .await
            {
                Ok(_) => Ok(true),
                Err(sqlx::Error::RowNotFound) => Ok(false),
                Err(err) => Err(err.into())

            }
    }

    pub async fn set_verification(
        pool: &PgPool,
        email: &str,
    ) -> Result<Verification, DatabaseError> {
        let account = Account::find(pool, email).await?;

        let verification =
            Verification::new(pool, super::verification::VerificationType::User, account.user_id)
                .await?;

        Ok(verification)
    }

    pub async fn find(pool: &PgPool, email: &str) -> Result<Account, DatabaseError> {
        Ok(sqlx::query_as!(
            Account,
            r#"SELECT user_id, organization_id, email, password, role AS "role!: Role", created_at, updated_at FROM account WHERE email = $1"#,
            email)
            .fetch_one(pool)
            .await?)
    }

    pub async fn clients(&self, pool: &PgPool) -> Result<Vec<Client>, DatabaseError> {
        Client::get_all_for_account(pool, self).await
    }

    pub async fn clients_and_machines(
        &self,
        pool: &PgPool,
    ) -> Result<Vec<(Client, Vec<Machine>)>, DatabaseError> {
        let mut clients = Vec::new();
        for client in self.clients(pool).await? {
            clients.push((
                client.clone(),
                Machine::get_all_for_client_id(pool, &client.client_id).await?,
            ));
        }
        Ok(clients)
    }

    pub async fn all_machines(&self, pool: &PgPool) -> Result<Vec<Machine>, DatabaseError> {
        let mut machines = Vec::new();
        for client in self.clients(pool).await? {
            let mut m = Machine::get_all_for_client_id(pool, &client.client_id).await?;
            machines.append(&mut m);
        }
        Ok(machines)
    }

    pub async fn all_avses(&self, pool: &PgPool) -> Result<Vec<Avs>, DatabaseError> {
        let mut avses = Vec::new();
        for machine in self.all_machines(pool).await? {
            let mut a = Avs::get_machines_avs_list(pool, machine.machine_id).await?;
            avses.append(&mut a);
        }
        Ok(avses)
    }

    pub async fn machines_and_avses(
        &self,
        pool: &PgPool,
    ) -> Result<Vec<(Machine, Vec<Avs>)>, DatabaseError> {
        let mut machines = Vec::new();
        for machine in self.all_machines(pool).await? {
            machines.push((
                machine.clone(),
                Avs::get_machines_avs_list(pool, machine.machine_id).await?,
            ));
        }
        Ok(machines)
    }

    pub async fn attach_client(
        &self,
        pool: &PgPool,
        client_id: &Address,
        machine_id: Uuid,
        name: &str,
    ) -> Result<(), DatabaseError> {
        Client::create(pool, self, client_id).await?;
        Machine::create(pool, client_id, name, machine_id, None).await?;
        Ok(())
    }

    pub async fn new(
        pool: &PgPool,
        organization: &Organization,
        email: &str,
        password: &str,
        role: Role,
    ) -> Result<Account, DatabaseError> {
        let now: NaiveDateTime = Utc::now().naive_utc();

        let account = sqlx::query_as!(
            Account,
            r#"INSERT INTO account (organization_id, email, password, role, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    RETURNING user_id, organization_id, email, password, role AS "role: _", created_at, updated_at"#,
            organization.organization_id,
            email,
            sha256::digest(password),
            role as Role,
            now,
            now
        )
        .fetch_one(pool)
        .await?;
        Ok(account)
    }

    pub async fn delete(&self, pool: &PgPool) -> Result<(), DatabaseError> {
        query!("DELETE FROM account WHERE email = $1", self.email).execute(pool).await?;
        Ok(())
    }

    pub async fn purge(pool: &PgPool) -> Result<(), DatabaseError> {
        query!("DELETE FROM account").execute(pool).await?;
        Ok(())
    }
}

// #[cfg(feature = "db_tests")]
// #[cfg(test)]
// mod tests {
//     use crate::ivynet_database::verification;
//
//     use super::*;
//
//     use sqlx::postgres::PgPoolOptions;
//
//     async fn setup_test_db() -> PgPool {
//         let database_url =
//             std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
//         PgPoolOptions::new()
//             .max_connections(5)
//             .connect(&database_url)
//             .await
//             .expect("Failed to connect to Postgres")
//     }
//
//     async fn setup_test_organization(pool: &PgPool) -> Organization {
//         // Implement this based on your Organization struct
//         Organization::new(pool, "Test Org", true).await.expect("Failed to create test
// organization")     }
//
//     #[tokio::test]
//     async fn test_account_crud_operations() {
//         let pool = setup_test_db().await;
//         let org = setup_test_organization(&pool).await;
//
//         // Test account creation
//         let email = "test@example.com";
//         let password = "password123";
//         let account = Account::new(&pool, &org, email, password, Role::User)
//             .await
//             .expect("Failed to create account");
//
//         assert_eq!(account.email, email);
//         assert_eq!(account.role, Role::User);
//
//         // Test account retrieval
//         let retrieved_account = Account::find(&pool, email).await.expect("Failed to find
// account");         assert_eq!(retrieved_account.email, account.email);
//
//         // Test account verification
//         let verified_account =
//             Account::verify(&pool, email, password).await.expect("Failed to verify account");
//         assert_eq!(verified_account.email, account.email);
//
//         // Test password change
//         let new_password = "newpassword123";
//         let updated_account =
//             account.set_password(&pool, new_password).await.expect("Failed to update password");
//         assert_ne!(updated_account.password, account.password);
//
//         // Test account deletion
//         account.delete(&pool).await.expect("Failed to delete account");
//         let account_exists =
//             Account::exists(&pool, email).await.expect("Failed to check account existence");
//         assert!(!account_exists);
//     }
//
//     #[tokio::test]
//     async fn test_account_role_methods() {
//         let admin_role = Role::Admin;
//         let user_role = Role::User;
//         let reader_role = Role::Reader;
//
//         assert!(admin_role.is_admin());
//         assert!(!user_role.is_admin());
//         assert!(!reader_role.is_admin());
//
//         assert!(admin_role.can_write());
//         assert!(user_role.can_write());
//         assert!(!reader_role.can_write());
//     }
//
//     #[tokio::test]
//     async fn test_account_node_operations() {
//         let pool = setup_test_db().await;
//         let org = setup_test_organization(&pool).await;
//
//         let account = Account::new(&pool, &org, "node_test@example.com", "password", Role::User)
//             .await
//             .expect("Failed to create account");
//
//         let node_id = Address::random();
//         let node_name = "Test Node";
//
//         // Test attaching a node
//         account.attach_node(&pool, &node_id, node_name).await.expect("Failed to attach node");
//
//         // Test retrieving nodes
//         let nodes = account.nodes(&pool).await.expect("Failed to retrieve nodes");
//         assert_eq!(nodes.len(), 1);
//         assert_eq!(nodes[0].node_id, node_id);
//         assert_eq!(nodes[0].name, node_name);
//
//         // Clean up
//         account.delete(&pool).await.expect("Failed to delete account");
//     }
//
//     #[tokio::test]
//     async fn test_account_verification() {
//         let pool = setup_test_db().await;
//         let org = setup_test_organization(&pool).await;
//
//         let email = "verify_test@example.com";
//         let account = Account::new(&pool, &org, email, "password", Role::User)
//             .await
//             .expect("Failed to create account");
//
//         let verification =
//             Account::set_verification(&pool, email).await.expect("Failed to set verification");
//
//         assert_eq!(verification.associated_id, account.user_id);
//         assert_eq!(verification.verification_type, verification::VerificationType::User);
//
//         // Clean up
//         account.delete(&pool).await.expect("Failed to delete account");
//     }
// }
