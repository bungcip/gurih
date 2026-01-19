use gurih_ir::Schema;
use serde_json::{json, Value};

pub struct PortalEngine;

impl PortalEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_navigation(&self, schema: &Schema) -> Result<Value, String> {
        let mut modules = vec![];

        for (module_name, module_def) in &schema.modules {
            let mut items = vec![];
            for entity_name in &module_def.entities {
                if let Some(entity) = schema.entities.get(entity_name) {
                     items.push(json!({
                        "label": entity.name,
                        "to": format!("/app/{}", entity.name),
                        "entity": entity.name
                    }));
                }
            }
            
            modules.push(json!({
                "label": module_name,
                "items": items
            }));
        }

        Ok(json!(modules))
    }
}
