pub mod alert_db;
pub mod alert_handler;
pub mod node_alerts_active;
pub mod node_alerts_historical;

#[cfg(test)]
mod test_alerts_db {
    use ivynet_alerts::Alert;
    use node_alerts_active::NewAlert;
    use sqlx::PgPool;
    use uuid::Uuid;

    use super::*;

    fn debug_uuid() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/node_alerts_active.sql",)
    )]
    #[ignore]
    async fn test_add_new_alert(pool: PgPool) {
        let alerts_active = node_alerts_active::NodeActiveAlert::get_all(&pool).await.unwrap();

        let num_alerts = alerts_active.len();
        let machine_id = Uuid::parse_str("dcbf22c7-9d96-47ac-bf06-62d6544e440d").unwrap();
        let alert_type = Alert::Custom {
            node_name: "test".to_string(),
            node_type: "test".to_string(),
            extra_data: serde_json::Value::String("test".to_string()),
        };
        let node_name = "test".to_string();

        let new_alert = NewAlert::new(machine_id, alert_type, node_name);
        node_alerts_active::NodeActiveAlert::insert_one(&pool, &new_alert).await.unwrap();

        let alerts_active = node_alerts_active::NodeActiveAlert::get_all(&pool).await.unwrap();

        assert_eq!(alerts_active.len(), num_alerts + 1);

        let new_db_alert =
            node_alerts_active::NodeActiveAlert::get(&pool, new_alert.id).await.unwrap().unwrap();

        assert_eq!(new_db_alert.alert_type, new_alert.alert_type);
        assert_eq!(new_db_alert.machine_id, new_alert.machine_id);
        assert_eq!(new_db_alert.node_name, new_alert.node_name);
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/node_alerts_active.sql",)
    )]
    #[ignore]
    async fn test_get_all_active_alerts(pool: PgPool) {
        let alert = node_alerts_active::NodeActiveAlert::get_all(&pool).await.unwrap();
        assert!(!alert.is_empty());
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/node_alerts_active.sql",)
    )]
    #[ignore]
    async fn test_get_alert_by_id(pool: PgPool) {
        let alert = node_alerts_active::NodeActiveAlert::get(&pool, debug_uuid()).await.unwrap();
        assert!(alert.is_some());
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/node_alerts_active.sql",)
    )]
    #[ignore]
    async fn test_get_alerts_by_org(pool: PgPool) {
        let alert = node_alerts_active::NodeActiveAlert::all_alerts_by_org(&pool, 1).await.unwrap();
        assert!(!alert.is_empty());
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/node_alerts_active.sql",)
    )]
    #[ignore]
    async fn test_get_alerts_by_machine(pool: PgPool) {
        let fixture_machine_id = Uuid::parse_str("dcbf22c7-9d96-47ac-bf06-62d6544e440d").unwrap();
        let alert =
            node_alerts_active::NodeActiveAlert::all_alerts_by_machine(&pool, fixture_machine_id)
                .await
                .unwrap();
        assert!(!alert.is_empty());
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/node_alerts_active.sql",)
    )]
    #[ignore]
    async fn test_acknowledge_alert(pool: PgPool) {
        let alert =
            node_alerts_active::NodeActiveAlert::get(&pool, debug_uuid()).await.unwrap().unwrap();
        node_alerts_active::NodeActiveAlert::acknowledge(&pool, alert.alert_id).await.unwrap();
        assert!(alert.acknowledged_at.is_none());
        let alert =
            node_alerts_active::NodeActiveAlert::get(&pool, debug_uuid()).await.unwrap().unwrap();
        assert!(alert.acknowledged_at.is_some());
    }

    #[sqlx::test(
        migrations = "../migrations",
        fixtures("../../fixtures/new_user_registration.sql", "../../fixtures/node_alerts_active.sql",)
    )]
    #[ignore]
    async fn test_resolve_alert(pool: PgPool) {
        let alert =
            node_alerts_active::NodeActiveAlert::get(&pool, debug_uuid()).await.unwrap().unwrap();
        node_alerts_active::NodeActiveAlert::resolve_alert(&pool, alert.alert_id).await.unwrap();

        // confirm that the alert is resolved in historical db
        let alert_historical =
            node_alerts_historical::NodeHistoryAlert::get(&pool, debug_uuid()).await.unwrap();
        assert!(alert_historical.is_some());

        // confirm that the alert is removed from active db
        let alert_resolved =
            node_alerts_active::NodeActiveAlert::get(&pool, debug_uuid()).await.unwrap();
        assert!(alert_resolved.is_none());
    }
}
