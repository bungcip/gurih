use crate::storage::Storage;
use gurih_ir::Schema;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub struct DashboardEngine;

impl Default for DashboardEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DashboardEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_ui_schema(&self, schema: &Schema, dashboard_name: &str) -> Result<Value, String> {
        let dashboard = schema
            .dashboards
            .get(dashboard_name)
            .ok_or("Dashboard not found")?;

        let widgets: Vec<Value> = dashboard
            .widgets
            .iter()
            .map(|w| {
                json!({
                    "name": w.name,
                    "type": w.widget_type,
                    "label": w.label,
                    "value": w.value,
                    "icon": w.icon
                })
            })
            .collect();

        Ok(json!({
            "name": dashboard.name,
            "title": dashboard.title,
            "layout": "Grid", // Implicit grid for now
            "widgets": widgets
        }))
    }

    pub async fn evaluate(
        &self,
        schema: &Schema,
        dashboard_name: &str,
        storage: &Arc<dyn Storage>,
    ) -> Result<Value, String> {
        let dashboard = schema
            .dashboards
            .get(dashboard_name)
            .ok_or("Dashboard not found")?;

        let mut widgets = vec![];
        for w in &dashboard.widgets {
            let mut evaluated_value = json!(null);

            if let Some(val_str) = &w.value {
                if val_str.starts_with("count:") {
                    // Parse count:Entity[k=v]
                    let rest = &val_str[6..];
                    let (entity, filter_str) = if let Some(idx) = rest.find('[') {
                        let entity = &rest[..idx];
                        let filter_part = &rest[idx + 1..rest.len() - 1]; // remove [ and ]
                        (entity, Some(filter_part))
                    } else {
                        (rest, None)
                    };

                    let mut filters = HashMap::new();
                    if let Some(f) = filter_str {
                        for pair in f.split(',') {
                            let parts: Vec<&str> = pair.split('=').collect();
                            if parts.len() == 2 {
                                filters.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
                            }
                        }
                    }

                    let count = storage.count(entity, filters).await?;
                    evaluated_value = json!(count);
                } else if val_str.starts_with("group:") {
                    // Parse group:Entity[field]
                    let rest = &val_str[6..];
                    let (entity, group_by) = if let Some(idx) = rest.find('[') {
                        let entity = &rest[..idx];
                        let group_part = &rest[idx + 1..rest.len() - 1];
                        (entity, Some(group_part))
                    } else {
                        (rest, None)
                    };

                    if let Some(field) = group_by {
                        let results = storage.aggregate(entity, field, HashMap::new()).await?;
                        let data: Vec<Value> = results
                            .iter()
                            .map(|(k, v)| json!({"label": k, "value": v}))
                            .collect();
                        evaluated_value = json!(data);
                    }
                } else {
                    // Static value
                    evaluated_value = json!(val_str);
                }
            }

            widgets.push(json!({
                "name": w.name,
                "type": w.widget_type,
                "label": w.label,
                "value": evaluated_value,
                "icon": w.icon
            }));
        }

        Ok(json!({
            "name": dashboard.name,
            "title": dashboard.title,
            "layout": "Grid",
            "widgets": widgets
        }))
    }
}
