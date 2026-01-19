use gurih_ir::Schema;
use serde_json::{Value, json};

pub struct PortalEngine;

impl Default for PortalEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PortalEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_navigation(&self, schema: &Schema) -> Result<Value, String> {
        // Prioritize explicit "MainMenu" or any menu if available
        if let Some(main_menu) = schema.menus.get("MainMenu") {
            return Ok(json!(Self::convert_menu_items(&main_menu.items)));
        } else if !schema.menus.is_empty() {
            // Fallback: collect all menus as sections?
            let mut modules = vec![];
            for menu in schema.menus.values() {
                modules.push(json!({
                    "label": menu.name,
                    "items": Self::convert_menu_items(&menu.items)
                }));
            }
            return Ok(json!(modules));
        }

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

    fn convert_menu_items(items: &[gurih_ir::MenuItemSchema]) -> Vec<Value> {
        items
            .iter()
            .map(|item| {
                let mut json_item = json!({
                    "label": item.label,
                    "icon": item.icon
                });

                if let Some(to) = &item.to {
                    json_item["to"] = json!(to);
                }

                if !item.children.is_empty() {
                    json_item["items"] = json!(Self::convert_menu_items(&item.children));
                }

                json_item
            })
            .collect()
    }
}
