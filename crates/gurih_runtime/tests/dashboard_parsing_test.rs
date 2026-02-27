use gurih_ir::{DashboardSchema, Schema, Symbol, WidgetSchema, WidgetType};
use gurih_runtime::dashboard::DashboardEngine;
use gurih_runtime::datastore::DataStore;
use gurih_runtime::store::MemoryDataStore;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_dashboard_parsing_logic() {
    let mut dashboards = HashMap::new();
    let dashboard_id = Symbol::from("ParsingDashboard");

    // Setup Widgets with special values
    dashboards.insert(
        dashboard_id,
        DashboardSchema {
            name: dashboard_id,
            title: "Parsing Test".to_string(),
            widgets: vec![
                WidgetSchema {
                    name: Symbol::from("CountWidget"),
                    widget_type: WidgetType::Stat,
                    value: Some("count:Entity[status=Active]".to_string()),
                    label: None,
                    icon: None,
                    roles: None,
                },
                WidgetSchema {
                    name: Symbol::from("GroupWidget"),
                    widget_type: WidgetType::Chart,
                    value: Some("group:Entity[category]".to_string()),
                    label: None,
                    icon: None,
                    roles: None,
                },
                WidgetSchema {
                    name: Symbol::from("StaticWidget"),
                    widget_type: WidgetType::Stat,
                    value: Some("static_value".to_string()),
                    label: None,
                    icon: None,
                    roles: None,
                },
            ],
        },
    );

    let mut schema = Schema::default();
    schema.dashboards = dashboards;

    let engine = DashboardEngine::new();
    let datastore = Arc::new(MemoryDataStore::new());

    // Seed Data
    datastore
        .insert("Entity", json!({"status": "Active", "category": "A"}))
        .await
        .unwrap();
    datastore
        .insert("Entity", json!({"status": "Active", "category": "B"}))
        .await
        .unwrap();
    datastore
        .insert("Entity", json!({"status": "Inactive", "category": "A"}))
        .await
        .unwrap();

    // Evaluate
    let result = engine
        .evaluate(&schema, dashboard_id, &(datastore as Arc<dyn DataStore>), &[])
        .await
        .unwrap();

    let widgets = result["widgets"].as_array().unwrap();

    // Check Count
    let count_widget = widgets.iter().find(|w| w["name"] == "CountWidget").unwrap();
    assert_eq!(count_widget["value"], 2, "Should count 2 Active entities");

    // Check Group
    let group_widget = widgets.iter().find(|w| w["name"] == "GroupWidget").unwrap();
    let group_data = group_widget["value"].as_array().unwrap();

    // Check Group Data content
    let count_a = group_data
        .iter()
        .find(|d| d["label"] == "A")
        .map(|d| d["value"].as_i64().unwrap())
        .unwrap_or(0);
    let count_b = group_data
        .iter()
        .find(|d| d["label"] == "B")
        .map(|d| d["value"].as_i64().unwrap())
        .unwrap_or(0);

    assert_eq!(count_a, 2, "Category A should have 2");
    assert_eq!(count_b, 1, "Category B should have 1");

    // Check Static
    let static_widget = widgets.iter().find(|w| w["name"] == "StaticWidget").unwrap();
    assert_eq!(static_widget["value"], "static_value");
}
