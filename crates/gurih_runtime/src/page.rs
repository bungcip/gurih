use gurih_ir::Schema;
use serde_json::{json, Value};

pub struct PageEngine;

impl PageEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_page_config(&self, schema: &Schema, entity_name: &str) -> Result<Value, String> {
        let entity = schema.entities.get(entity_name).ok_or("Entity not found")?;

        let columns: Vec<Value> = entity.fields.iter().map(|f| {
            json!({
                "key": f.name,
                "label": f.name, // TODO: Humanize label
                "type": format!("{:?}", f.field_type)
            })
        }).collect();

        Ok(json!({
            "title": entity.name,
            "entity": entity_name,
            "layout": "TableView",
            "columns": columns,
            "actions": ["create", "edit", "delete"]
        }))
    }
}
