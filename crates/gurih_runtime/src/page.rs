use gurih_ir::Schema;
use serde_json::{Value, json};

pub struct PageEngine;

impl Default for PageEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PageEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_page_config(
        &self,
        schema: &Schema,
        entity_name: &str,
    ) -> Result<Value, String> {
        // 1. Try explicit Page definition
        if let Some(page) = schema.pages.get(entity_name) {
            match &page.content {
                gurih_ir::PageContentSchema::Datatable(dt) => {
                    let columns: Vec<Value> = dt
                        .columns
                        .iter()
                        .map(|c| {
                            json!({
                                "key": c.field,
                                "label": c.label,
                                "type": "String" // Placeholder, ideally lookup field type from entity
                            })
                        })
                        .collect();

                    let actions: Vec<String> = dt.actions.iter().map(|a| a.label.clone()).collect();

                    return Ok(json!({
                       "title": page.title,
                       "entity": dt.entity,
                       "layout": "TableView",
                       "columns": columns,
                       "actions": actions
                    }));
                }
                gurih_ir::PageContentSchema::Dashboard(name) => {
                    let engine = crate::dashboard::DashboardEngine::new();
                    return engine.generate_ui_schema(schema, name);
                }
                gurih_ir::PageContentSchema::Form(name) => {
                    let engine = crate::form::FormEngine::new();
                    return engine.generate_ui_schema(schema, name);
                }
                gurih_ir::PageContentSchema::None => {
                    return Ok(json!({
                        "title": page.title,
                        "layout": "Empty"
                    }));
                }
            }
        }

        // 2. Try direct Dashboard
        if schema.dashboards.contains_key(entity_name) {
            let engine = crate::dashboard::DashboardEngine::new();
            return engine.generate_ui_schema(schema, entity_name);
        }

        // 3. Try direct Form
        if schema.forms.contains_key(entity_name) {
            let engine = crate::form::FormEngine::new();
            return engine.generate_ui_schema(schema, entity_name);
        }

        let entity = schema
            .entities
            .get(entity_name)
            .ok_or("Entity, Page, or Dashboard not found")?;

        let columns: Vec<Value> = entity
            .fields
            .iter()
            .map(|f| {
                json!({
                    "key": f.name,
                    "label": f.name, // TODO: Humanize label
                    "type": format!("{:?}", f.field_type)
                })
            })
            .collect();

        Ok(json!({
            "title": entity.name,
            "entity": entity_name,
            "layout": "TableView",
            "columns": columns,
            "actions": ["create", "edit", "delete"]
        }))
    }
}
