use crate::datastore::DataStore;
use gurih_ir::{Schema, Symbol};
use serde_json::{Value, json};
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

    pub fn generate_ui_schema(&self, schema: &Schema, dashboard: Symbol) -> Result<Value, String> {
        let dashboard = schema.dashboards.get(&dashboard).ok_or("Dashboard not found")?;

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
        dashboard: Symbol,
        datastore: &Arc<dyn DataStore>,
        user_roles: &[String],
    ) -> Result<Value, String> {
        let dashboard = schema.dashboards.get(&dashboard).ok_or("Dashboard not found")?;

        let mut widgets = vec![];
        for w in &dashboard.widgets {
            if let Some(required_roles) = &w.roles
                && !required_roles.is_empty()
                && !required_roles.iter().any(|r| user_roles.contains(r))
            {
                continue;
            }

            let mut evaluated_value = json!(null);

            if let Some(val_str) = &w.value {
                if let Some(rest) = val_str.strip_prefix("count:") {
                    let (entity, filter_str) = Self::parse_directive(rest);

                    let mut filters = HashMap::new();
                    if let Some(f) = filter_str {
                        for pair in f.split(',') {
                            let parts: Vec<&str> = pair.split('=').collect();
                            if parts.len() == 2 {
                                filters.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
                            }
                        }
                    }

                    let count = datastore.count(entity, filters).await?;
                    evaluated_value = json!(count);
                } else if let Some(rest) = val_str.strip_prefix("group:") {
                    let (entity, group_by) = Self::parse_directive(rest);

                    if let Some(field) = group_by {
                        let results = datastore.aggregate(entity, field, HashMap::new()).await?;
                        let data: Vec<Value> = results.iter().map(|(k, v)| json!({"label": k, "value": v})).collect();
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

    fn parse_directive(rest: &str) -> (&str, Option<&str>) {
        if let Some(idx) = rest.find('[') {
            let entity = &rest[..idx];
            let inner = &rest[idx + 1..rest.len() - 1]; // remove [ and ]
            (entity, Some(inner))
        } else {
            (rest, None)
        }
    }
}
