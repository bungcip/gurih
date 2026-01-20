use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Schema {
    pub name: String,
    pub version: String,
    pub database: Option<DatabaseSchema>,         // Added
    pub storages: HashMap<String, StorageSchema>, // Added
    pub modules: HashMap<String, ModuleSchema>,
    pub entities: HashMap<String, EntitySchema>,
    pub tables: HashMap<String, TableSchema>, // Added
    pub workflows: HashMap<String, WorkflowSchema>,
    pub forms: HashMap<String, FormSchema>,
    pub actions: HashMap<String, ActionLogic>, // Added
    pub permissions: HashMap<String, PermissionSchema>,

    // New fields
    // ... existing code ...
    pub layouts: HashMap<String, LayoutSchema>,
    pub menus: HashMap<String, MenuSchema>,
    pub routes: HashMap<String, RouteSchema>,
    pub pages: HashMap<String, PageSchema>,
    pub dashboards: HashMap<String, DashboardSchema>,
    pub serial_generators: HashMap<String, SerialGeneratorSchema>,
    pub prints: HashMap<String, PrintSchema>,
    pub queries: HashMap<String, QuerySchema>, // Added
}

// ... existing code ...

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatatableSchema {
    pub entity: Option<String>, // Changed to Option
    pub query: Option<String>,  // Added
    pub columns: Vec<DatatableColumnSchema>,
    pub actions: Vec<ActionSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySchema {
    pub name: String,
    pub root_entity: String,
    pub query_type: QueryType, // Added
    pub selections: Vec<QuerySelection>,
    pub formulas: Vec<QueryFormula>,
    pub filters: Vec<Expression>, // Added
    pub joins: Vec<QueryJoin>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryType {
    Nested,
    Flat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySelection {
    pub field: String,
    pub alias: Option<String>, // Added
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFormula {
    pub name: String,
    pub expression: Expression,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Expression {
    Field(String),
    Literal(f64), // Using f64 for simplicity as implied by "excel like"
    StringLiteral(String),
    FunctionCall {
        name: String,
        args: Vec<Expression>,
    },
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
    Grouping(Box<Expression>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryJoin {
    pub target_entity: String,
    pub selections: Vec<QuerySelection>,
    pub formulas: Vec<QueryFormula>,
    pub joins: Vec<QueryJoin>,
}

// ... existing code ...
#[cfg(test)]
mod tests {
    #[test]
    fn test_serialization() {
        // ... (simplified test)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub db_type: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSchema {
    pub name: String,
    pub driver: String,
    pub location: Option<String>,
    pub props: HashMap<String, String>,
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
    pub seeds: Option<Vec<HashMap<String, String>>>,
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
    pub serial_generator: Option<String>,
    pub storage: Option<String>,
    pub resize: Option<String>,
    pub filetype: Option<String>,
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
    Password,
    Enum(Vec<String>),
    Relation, // One-to-One or Many-to-One usually
    Photo,
    File,
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
    pub verb: String, // Added: GET, POST, DELETE, etc.
    pub path: String,
    pub action: String, // Renamed from 'to'
    pub layout: Option<String>,
    pub permission: Option<String>,
    pub children: Vec<RouteSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSchema {
    pub label: String,
    pub to: Option<String>,
    pub method: Option<String>, // Added
    pub icon: Option<String>,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLogic {
    pub name: String,
    pub params: Vec<String>,
    pub steps: Vec<ActionStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStep {
    pub step_type: String,
    pub target: String,
    pub args: HashMap<String, String>,
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
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialGeneratorSchema {
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
pub struct DatatableColumnSchema {
    pub field: String,
    pub label: String,
}
