use gurih_ir::{DashboardSchema, Schema, Symbol, WidgetSchema, WidgetType};
use gurih_runtime::dashboard::DashboardEngine;
use gurih_runtime::store::MemoryDataStore;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_dashboard_role_filtering() {
    let mut dashboards = HashMap::new();
    let dashboard_id = Symbol::from("MyDashboard");
    dashboards.insert(
        dashboard_id,
        DashboardSchema {
            name: dashboard_id,
            title: "Test Dashboard".to_string(),
            widgets: vec![
                WidgetSchema {
                    name: Symbol::from("AdminWidget"),
                    widget_type: WidgetType::Stat,
                    label: Some("Admin Only".to_string()),
                    value: Some("100".to_string()),
                    icon: None,
                    roles: Some(vec!["Admin".to_string()]),
                },
                WidgetSchema {
                    name: Symbol::from("UserWidget"),
                    widget_type: WidgetType::Stat,
                    label: Some("User Only".to_string()),
                    value: Some("50".to_string()),
                    icon: None,
                    roles: Some(vec!["User".to_string()]),
                },
                WidgetSchema {
                    name: Symbol::from("PublicWidget"),
                    widget_type: WidgetType::Stat,
                    label: Some("Public".to_string()),
                    value: Some("10".to_string()),
                    icon: None,
                    roles: None,
                },
            ],
        },
    );

    let schema = Schema {
        dashboards,
        ..Default::default()
    };

    let engine = DashboardEngine::new();
    let datastore: Arc<dyn gurih_runtime::datastore::DataStore> = Arc::new(MemoryDataStore::new());

    // Test Admin Role
    let admin_result = engine
        .evaluate(&schema, dashboard_id, &datastore, &["Admin".to_string()])
        .await
        .unwrap();

    let admin_widgets = admin_result["widgets"].as_array().unwrap();
    assert_eq!(admin_widgets.len(), 2, "Admin should see 2 widgets");
    assert!(admin_widgets.iter().any(|w| w["name"] == "AdminWidget"));
    assert!(admin_widgets.iter().any(|w| w["name"] == "PublicWidget"));
    assert!(!admin_widgets.iter().any(|w| w["name"] == "UserWidget"));

    // Test User Role
    let user_result = engine
        .evaluate(&schema, dashboard_id, &datastore, &["User".to_string()])
        .await
        .unwrap();

    let user_widgets = user_result["widgets"].as_array().unwrap();
    assert_eq!(user_widgets.len(), 2, "User should see 2 widgets");
    assert!(user_widgets.iter().any(|w| w["name"] == "UserWidget"));
    assert!(user_widgets.iter().any(|w| w["name"] == "PublicWidget"));
    assert!(!user_widgets.iter().any(|w| w["name"] == "AdminWidget"));

    // Test No Role (Public only)
    let public_result = engine.evaluate(&schema, dashboard_id, &datastore, &[]).await.unwrap();

    let public_widgets = public_result["widgets"].as_array().unwrap();
    assert_eq!(public_widgets.len(), 1, "Public should see 1 widget");
    assert!(public_widgets.iter().any(|w| w["name"] == "PublicWidget"));
}
