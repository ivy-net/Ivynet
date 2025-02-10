use serde::Serialize;

pub mod alerts_active;
pub mod alerts_historical;

#[derive(Debug, Clone, Copy, Serialize, Eq, PartialEq)]
pub enum AlertType {
    DEBUG = 0,
    ERROR = 1,
}

impl From<i64> for AlertType {
    fn from(value: i64) -> Self {
        match value {
            0 => AlertType::DEBUG,
            1 => AlertType::ERROR,
            _ => panic!("Unknown alert type: {}", value),
        }
    }
}

#[cfg(test)]
#[ignore]
mod test_alerts_db {
    use ivynet_core::ethers::types::Address;
    use sqlx::PgPool;
    use uuid::Uuid;

    use super::*;

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/alerts_active.sql",)
    )]
    async fn test_add_new_alert(pool: PgPool) {
        let alerts_active = alerts_active::ActiveAlert::get_all(&pool).await.unwrap();

        let num_alerts = alerts_active.len();

        let new_alert = alerts_active::NewAlert {
            alert_type: AlertType::ERROR,
            machine_id: Uuid::parse_str("dcbf22c7-9d96-47ac-bf06-62d6544e440d").unwrap(),
            organization_id: 1,
            client_id: Address::from_slice(&[1; 20]),
            node_name: "test".to_string(),
            created_at: chrono::Utc::now().naive_utc(),
        };

        alerts_active::ActiveAlert::record_new(&pool, &new_alert).await.unwrap();

        let alerts_active = alerts_active::ActiveAlert::get_all(&pool).await.unwrap();

        assert_eq!(alerts_active.len(), num_alerts + 1);

        let new_db_alert =
            alerts_active::ActiveAlert::get(&pool, num_alerts as i64 + 1).await.unwrap().unwrap();

        assert_eq!(new_db_alert.alert_type, new_alert.alert_type);
        assert_eq!(new_db_alert.machine_id, new_alert.machine_id);
        assert_eq!(new_db_alert.organization_id, new_alert.organization_id);
        assert_eq!(new_db_alert.client_id, new_alert.client_id);
        assert_eq!(new_db_alert.node_name, new_alert.node_name);
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/alerts_active.sql",)
    )]
    async fn test_get_all_active_alerts(pool: PgPool) {
        let alert = alerts_active::ActiveAlert::get_all(&pool).await.unwrap();
        assert!(!alert.is_empty());
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/alerts_active.sql",)
    )]
    async fn test_get_alert_by_id(pool: PgPool) {
        let alert = alerts_active::ActiveAlert::get(&pool, 1).await.unwrap();
        assert!(alert.is_some());
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/alerts_active.sql",)
    )]
    async fn test_get_alerts_by_org(pool: PgPool) {
        let alert = alerts_active::ActiveAlert::all_alerts_by_org(&pool, 1).await.unwrap();
        assert!(!alert.is_empty());
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/alerts_active.sql",)
    )]
    async fn test_get_alerts_by_machine(pool: PgPool) {
        let fixture_machine_id = Uuid::parse_str("dcbf22c7-9d96-47ac-bf06-62d6544e440d").unwrap();
        let alert = alerts_active::ActiveAlert::all_alerts_by_machine(&pool, fixture_machine_id)
            .await
            .unwrap();
        assert!(!alert.is_empty());
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/alerts_active.sql",)
    )]
    async fn test_acknowledge_alert(pool: PgPool) {
        let alert = alerts_active::ActiveAlert::get(&pool, 1).await.unwrap().unwrap();
        alerts_active::ActiveAlert::acknowledge(&pool, alert.alert_id).await.unwrap();
        assert!(alert.acknowledged_at.is_none());
        let alert = alerts_active::ActiveAlert::get(&pool, 1).await.unwrap().unwrap();
        assert!(alert.acknowledged_at.is_some());
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/alerts_active.sql",)
    )]
    async fn test_resolve_alert(pool: PgPool) {
        let alert = alerts_active::ActiveAlert::get(&pool, 1).await.unwrap().unwrap();
        alerts_active::ActiveAlert::resolve_alert(&pool, alert.alert_id).await.unwrap();

        // confirm that the alert is resolved in historical db
        let alert_historical = alerts_historical::HistoryAlert::get(&pool, 1).await.unwrap();
        assert!(alert_historical.is_some());

        // confirm that the alert is removed from active db
        let alert_resolved = alerts_active::ActiveAlert::get(&pool, 1).await.unwrap();
        assert!(alert_resolved.is_none());
    }
}
