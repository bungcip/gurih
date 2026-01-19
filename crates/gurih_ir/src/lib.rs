use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Schema {
    pub name: String,
    pub version: String,
    pub database: Option<DatabaseSchema>, // Added
    pub modules: HashMap<String, ModuleSchema>,
    pub entities: HashMap<String, EntitySchema>,
    pub tables: HashMap<String, TableSchema>, // Added
    pub workflows: HashMap<String, WorkflowSchema>,
    pub forms: HashMap<String, FormSchema>,
    pub permissions: HashMap<String, PermissionSchema>,

    // New fields
    pub layouts: HashMap<String, LayoutSchema>,
    pub menus: HashMap<String, MenuSchema>,
    pub routes: HashMap<String, RouteSchema>,
    pub pages: HashMap<String, PageSchema>,
    pub dashboards: HashMap<String, DashboardSchema>,
    pub serials: HashMap<String, SerialSchema>,
    pub prints: HashMap<String, PrintSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub db_type: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSchema {
    pub name: String,
    pub entities: Vec<String>, // Entity names
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySchema {
    pub name: String,
    pub fields: Vec<FieldSchema>,
    pub relationships: Vec<RelationshipSchema>,
    pub options: HashMap<String, String>, // is_submittable, track_changes, etc
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: String,
    pub type_name: String,
    pub props: HashMap<String, String>,
    pub primary: bool,
    pub unique: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSchema {
    pub name: String,
    pub target_entity: String,
    pub rel_type: String, // belongs_to, has_many, has_one
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub unique: bool,
    pub default: Option<String>,
    pub references: Option<String>, // Entity name for relations
    pub serial: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Text, // Long string
    Integer,
    Float,
    Boolean,
    Date,
    DateTime,
    Enum(Vec<String>),
    Relation, // One-to-One or Many-to-One usually
              // JSON,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSchema {
    pub name: String,
    pub entity: String,
    pub initial_state: String,
    pub states: Vec<String>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub name: String,
    pub from: String,
    pub to: String,
    pub required_permission: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormSchema {
    pub name: String,
    pub entity: String,
    pub sections: Vec<FormSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormSection {
    pub title: String,
    pub fields: Vec<String>, // Field names
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSchema {
    pub name: String,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSchema {
    pub name: String,
    // Simplified specific props
    pub header_enabled: bool,
    pub sidebar_enabled: bool,
    pub footer: Option<String>,
    pub props: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuSchema {
    pub name: String,
    pub items: Vec<MenuItemSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItemSchema {
    pub label: String,
    pub to: Option<String>,
    pub icon: Option<String>,
    pub children: Vec<MenuItemSchema>, // recursive
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSchema {
    // Flattened or tree? Keeping it tree-like might be useful for frontend.
    // But for IR, maybe flattened listing is easier?
    // Let's stick to list of top-level groups or routes.
    // Actually, `Schema` has `routes` as HashMap. The key is path?
    // DSL has `routes { route ... }`. It's a collection.
    // Maybe `RouteSchema` represents a single route entry.
    pub path: String,
    pub to: String, // Page or Dashboard name
    pub layout: Option<String>,
    pub permission: Option<String>,
    pub children: Vec<RouteSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSchema {
    pub name: String,
    pub title: String,
    pub content: PageContentSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageContentSchema {
    Datatable(DatatableSchema),
    Form(String),      // Form name
    Dashboard(String), // Dashboard name
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatatableSchema {
    pub entity: String,
    pub columns: Vec<DatatableColumnSchema>,
    pub actions: Vec<ActionSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatatableColumnSchema {
    pub field: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSchema {
    pub label: String,
    pub to: Option<String>,
    pub icon: Option<String>,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSchema {
    pub name: String,
    pub title: String,
    pub widgets: Vec<WidgetSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetSchema {
    pub name: String,
    pub widget_type: String,
    pub label: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialSchema {
    pub name: String,
    pub prefix: Option<String>,
    pub digits: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintSchema {
    pub name: String,
    pub entity: String,
    pub title: String,
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_serialization() {
        // ... (simplified test)
    }
}
