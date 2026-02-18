use gurih_ir::{Schema, Symbol};
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
        if let Some(main_menu) = schema.menus.get(&Symbol::from("MainMenu")) {
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
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        // 1. Try to find path in routes tree
        for route in schema.routes.values() {
            #[allow(clippy::collapsible_if)]
            if let Some(target_action) = Self::walk_route(route, &parts) {
                if target_action != Symbol::from("") {
                    let res = Self::resolve_entity_from_target(schema, target_action.as_str());
                    return Some(res);
                }
            }
        }

        // 2. Fallback: search strings in path
        for page_name in schema.pages.keys() {
            let path_lower = path.to_lowercase();
            if path_lower.contains(&page_name.as_str().to_lowercase()) {
                return Some(Self::resolve_entity_from_target(schema, page_name.as_str()));
            }
        }

        for entity_name in schema.entities.keys() {
            if path.to_lowercase().contains(&entity_name.as_str().to_lowercase()) {
                return Some(entity_name.as_str().to_string());
            }
        }

        None
    }

    fn walk_route(route: &gurih_ir::RouteSchema, segments: &[&str]) -> Option<Symbol> {
        if segments.is_empty() {
            return None;
        }

        let current_segment = segments[0];
        let current_path = route.path.trim_start_matches('/');

        // If current route matches the first segment
        if current_path == current_segment || (route.path == "/" && current_segment.is_empty()) {
            let remaining = &segments[1..];
            if remaining.is_empty() {
                // If we've consumed all segments, return this route's action
                // or check if there's a child with path "/" that has an action
                if route.action != Symbol::from("") {
                    return Some(route.action);
                }
                for child in &route.children {
                    if child.path == "/" && child.action != Symbol::from("") {
                        return Some(child.action);
                    }
                }
                return Some(Symbol::from(""));
            }

            // Otherwise, try to find match in children
            for child in &route.children {
                #[allow(clippy::collapsible_if)]
                if let Some(res) = Self::walk_route(child, remaining) {
                    if res != Symbol::from("") || remaining.len() == 1 {
                        return Some(res);
                    }
                }
            }
        } else if route.path == "/" {
            // Root group might match the same segment if it's just a prefix "/"
            for child in &route.children {
                #[allow(clippy::collapsible_if)]
                if let Some(res) = Self::walk_route(child, segments) {
                    if res != Symbol::from("") {
                        return Some(res);
                    }
                }
            }
        }

        None
    }

    fn resolve_entity_from_target(schema: &Schema, target: &str) -> String {
        // If target is an entity name, return it
        if schema.entities.contains_key(&Symbol::from(target)) {
            return target.to_string();
        }

        // If target is a page name, return the entity it's for
        if let Some(page) = schema.pages.get(&Symbol::from(target)) {
            match &page.content {
                gurih_ir::PageContentSchema::Datatable(dt) => {
                    if let Some(param) = &dt.entity {
                        return param.as_str().to_string();
                    }
                    if let Some(q_name) = &dt.query
                        && let Some(q) = schema.queries.get(q_name)
                    {
                        return q.root_entity.as_str().to_string();
                    }
                    return "".to_string();
                }
                gurih_ir::PageContentSchema::Form(form_name) => {
                    if let Some(form) = schema.forms.get(form_name) {
                        return form.entity.as_str().to_string();
                    }
                }
                _ => {}
            }
        }

        target.to_string()
    }
}
