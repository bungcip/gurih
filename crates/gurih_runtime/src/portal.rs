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
            let mut modules = vec![];
            let mut top_level_items = vec![];

            for item in &main_menu.items {
                if item.children.is_empty() && item.to.is_some() {
                    top_level_items.push(Self::convert_single_item(item, schema));
                } else {
                    modules.push(json!({
                        "label": item.label,
                        "items": Self::convert_menu_items(&item.children, schema)
                    }));
                }
            }

            if !top_level_items.is_empty() {
                modules.insert(
                    0,
                    json!({
                        "label": "General",
                        "items": top_level_items
                    }),
                );
            }
            return Ok(json!(modules));
        } else if !schema.menus.is_empty() {
            // Fallback: collect all menus as sections
            let mut modules = vec![];
            for menu in schema.menus.values() {
                modules.push(json!({
                    "label": menu.name,
                    "items": Self::convert_menu_items(&menu.items, schema)
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

    fn convert_menu_items(items: &[gurih_ir::MenuItemSchema], schema: &Schema) -> Vec<Value> {
        items
            .iter()
            .map(|item| {
                if item.children.is_empty() {
                    Self::convert_single_item(item, schema)
                } else {
                    json!({
                        "label": item.label,
                        "icon": item.icon,
                        "items": Self::convert_menu_items(&item.children, schema)
                    })
                }
            })
            .collect()
    }

    fn convert_single_item(item: &gurih_ir::MenuItemSchema, schema: &Schema) -> Value {
        let mut json_item = json!({
            "label": item.label,
            "icon": item.icon
        });

        if let Some(to) = &item.to {
            json_item["to"] = json!(to);
            if let Some(entity) = Self::resolve_entity_from_path(schema, to) {
                json_item["entity"] = json!(entity);
            }
        }

        json_item
    }

    fn resolve_entity_from_path(schema: &Schema, path: &str) -> Option<String> {
        let mut target = None;
        // Direct match in routes
        if let Some(r) = schema.routes.get(path) {
            if !r.to.is_empty() {
                target = Some(r.to.clone());
            } else {
                // If it's a group, check child "/"
                for child in &r.children {
                    if child.path == "/" && !child.to.is_empty() {
                        target = Some(child.to.clone());
                        break;
                    }
                }
            }
        }

        if let Some(t) = target {
            return Some(Self::resolve_entity_from_target(schema, &t));
        }

        // Fallback: search strings in path
        for (page_name, _) in &schema.pages {
            let path_lower = path.to_lowercase();
            if path_lower.contains(&page_name.to_lowercase()) {
                return Some(Self::resolve_entity_from_target(schema, page_name));
            }
        }

        for entity_name in schema.entities.keys() {
            if path.to_lowercase().contains(&entity_name.to_lowercase()) {
                return Some(entity_name.clone());
            }
        }

        None
    }

    fn resolve_entity_from_target(schema: &Schema, target: &str) -> String {
        // If target is an entity name, return it
        if schema.entities.contains_key(target) {
            return target.to_string();
        }

        // If target is a page name, return the entity it's for
        if let Some(page) = schema.pages.get(target) {
            match &page.content {
                gurih_ir::PageContentSchema::Datatable(dt) => return dt.entity.clone(),
                gurih_ir::PageContentSchema::Form(form_name) => {
                    if let Some(form) = schema.forms.get(form_name) {
                        return form.entity.clone();
                    }
                }
                _ => {}
            }
        }

        target.to_string()
    }
}
