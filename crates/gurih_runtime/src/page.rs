use gurih_ir::{Schema, Symbol};
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

    pub fn generate_page_config(&self, schema: &Schema, entity_name: &str) -> Result<Value, String> {
        // 1. Try explicit Page definition by name
        let mut target_page = schema.pages.get(&Symbol::from(entity_name));

        // 2. Fallback: Search for a page that targets this entity (e.g. Employee -> EmployeeList)
        if target_page.is_none() {
            target_page = schema.pages.values().find(|p| match &p.content {
                gurih_ir::PageContentSchema::Datatable(dt) => {
                    dt.entity.as_ref().map(|s| s.as_str()) == Some(entity_name)
                }
                _ => false,
            });
        }

        if let Some(page) = target_page {
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

                    let actions: Vec<Value> = dt
                        .actions
                        .iter()
                        .map(|a| {
                            json!({
                                "label": a.label,
                                "to": a.to,
                                "icon": a.icon,
                                "variant": a.variant,
                                "method": a.method
                            })
                        })
                        .collect();

                    return Ok(json!({
                        "title": page.title,
                        "entity": dt.query
                            .or(dt.entity)
                            .unwrap_or(Symbol::from("")),
                        "layout": "TableView",
                       "columns": columns,
                       "actions": actions
                    }));
                }
                gurih_ir::PageContentSchema::Dashboard(name) => {
                    let engine = crate::dashboard::DashboardEngine::new();
                    return engine.generate_ui_schema(schema, name.as_str());
                }
                gurih_ir::PageContentSchema::Form(name) => {
                    let engine = crate::form::FormEngine::new();
                    return engine.generate_ui_schema(schema, name.as_str());
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
        if schema.dashboards.contains_key(&Symbol::from(entity_name)) {
            let engine = crate::dashboard::DashboardEngine::new();
            return engine.generate_ui_schema(schema, entity_name);
        }

        // 3. Try direct Form
        if schema.forms.contains_key(&Symbol::from(entity_name)) {
            let engine = crate::form::FormEngine::new();
            return engine.generate_ui_schema(schema, entity_name);
        }

        let entity = schema
            .entities
            .get(&Symbol::from(entity_name))
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
            "actions": [
                { "label": "Create", "variant": "primary" },
                { "label": "Edit" },
                { "label": "Delete", "variant": "danger" }
            ]
        }))
    }
}
