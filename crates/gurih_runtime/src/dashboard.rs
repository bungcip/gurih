use gurih_ir::Schema;
use serde_json::{json, Value};

pub struct DashboardEngine;

impl DashboardEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_ui_schema(&self, schema: &Schema, dashboard_name: &str) -> Result<Value, String> {
        let dashboard = schema.dashboards.get(dashboard_name).ok_or("Dashboard not found")?;

        let widgets: Vec<Value> = dashboard.widgets.iter().map(|w| {
            json!({
                "name": w.name,
                "type": w.widget_type,
                "label": w.label,
                "value": w.value,
                // "icon": w.icon // TODO: Add icon to WidgetSchema/WidgetDef if missing
            })
        }).collect();

        Ok(json!({
            "name": dashboard.name,
            "title": dashboard.title,
            "layout": "Grid", // Implicit grid for now
            "widgets": widgets
        }))
    }
}
