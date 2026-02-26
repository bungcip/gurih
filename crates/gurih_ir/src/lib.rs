use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub use symbol_table::GlobalSymbol as Symbol;

pub mod utils;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub name: Symbol,
    pub version: String,
    pub database: Option<DatabaseSchema>,
    pub storages: HashMap<Symbol, StorageSchema>,
    pub modules: HashMap<Symbol, ModuleSchema>,
    pub entities: HashMap<Symbol, EntitySchema>,
    pub tables: HashMap<Symbol, TableSchema>,
    pub workflows: HashMap<Symbol, WorkflowSchema>,
    pub forms: HashMap<Symbol, FormSchema>,
    pub actions: HashMap<Symbol, ActionLogic>,
    pub permissions: HashMap<Symbol, PermissionSchema>,

    // New fields
    // ... existing code ...
    pub layouts: HashMap<Symbol, LayoutSchema>,
    pub menus: HashMap<Symbol, MenuSchema>,
    pub routes: HashMap<String, RouteSchema>, // Routes are paths, maybe keep String? Or Symbol? Path can be a key. Let's keep String for path keys for now if they are URL paths.
    pub pages: HashMap<Symbol, PageSchema>,
    pub dashboards: HashMap<Symbol, DashboardSchema>,
    pub serial_generators: HashMap<Symbol, SerialGeneratorSchema>,
    pub prints: HashMap<Symbol, PrintSchema>,
    pub queries: HashMap<Symbol, QuerySchema>,
    pub rules: HashMap<Symbol, RuleSchema>,
    pub posting_rules: HashMap<Symbol, PostingRuleSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostingRuleSchema {
    pub name: Symbol,
    pub source_entity: Symbol,
    pub description_expr: Expression,
    pub date_expr: Expression,
    pub lines: Vec<PostingLineSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostingLineSchema {
    pub account: Symbol,
    pub debit_expr: Option<Expression>,
    pub credit_expr: Option<Expression>,
    #[serde(default)]
    pub fields: HashMap<Symbol, Expression>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSchema {
    pub name: Symbol,
    pub on_event: Symbol,
    pub assertion: Expression,
    pub message: String,
}

// ... existing code ...

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatatableSchema {
    pub entity: Option<Symbol>,
    pub query: Option<Symbol>,
    pub columns: Vec<DatatableColumnSchema>,
    pub actions: Vec<ActionSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySchema {
    pub name: Symbol,
    pub params: Vec<Symbol>,
    pub root_entity: Symbol,
    pub query_type: QueryType,
    pub selections: Vec<QuerySelection>,
    pub formulas: Vec<QueryFormula>,
    pub filters: Vec<Expression>,
    pub joins: Vec<QueryJoin>,
    pub group_by: Vec<Symbol>,
    pub hierarchy: Option<HierarchySchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchySchema {
    pub parent_field: Symbol,
    pub rollup_fields: Vec<Symbol>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryType {
    Nested,
    Flat,
    Hierarchy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuerySelection {
    pub field: Symbol,
    pub alias: Option<Symbol>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFormula {
    pub name: Symbol,
    pub expression: Expression,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Expression {
    Field(Symbol),
    Literal(f64),
    StringLiteral(String),
    BoolLiteral(bool),
    FunctionCall {
        name: Symbol,
        args: Vec<Expression>,
    },
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expression>,
    },
    Grouping(Box<Expression>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Neq,
    Gt,
    Lt,
    Gte,
    Lte,
    And,
    Or,
    Like,
    ILike,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not,
    Neg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryJoin {
    pub target_entity: Symbol,
    pub selections: Vec<QuerySelection>,
    pub formulas: Vec<QueryFormula>,
    pub joins: Vec<QueryJoin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub db_type: DatabaseType,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    Postgres,
    Sqlite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSchema {
    pub name: Symbol,
    pub driver: StorageDriver,
    pub location: Option<String>,
    pub props: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageDriver {
    S3,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSchema {
    pub name: Symbol,
    pub entities: Vec<Symbol>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySchema {
    pub name: Symbol,
    pub table_name: Symbol,
    pub fields: Vec<FieldSchema>,
    pub relationships: Vec<RelationshipSchema>,
    pub options: HashMap<String, String>,
    pub seeds: Option<Vec<HashMap<String, String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub name: Symbol,
    pub columns: Vec<ColumnSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: Symbol,
    pub type_name: ColumnType,
    pub props: HashMap<String, String>,
    pub primary: bool,
    pub unique: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    Serial,
    Varchar,
    Text,
    Integer,
    Float,
    Boolean,
    Date,
    Timestamp,
    Uuid,
    Json,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSchema {
    pub name: Symbol,
    pub target_entity: Symbol,
    pub rel_type: RelationshipType,
    #[serde(default = "default_ownership")]
    pub ownership: Ownership,
}

fn default_ownership() -> Ownership {
    Ownership::Reference
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    BelongsTo,
    HasMany,
    HasOne,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Ownership {
    Reference,
    Composition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: Symbol,
    pub field_type: FieldType,
    pub required: bool,
    pub unique: bool,
    pub default: Option<String>,
    pub references: Option<Symbol>,
    pub serial_generator: Option<Symbol>,
    pub storage: Option<Symbol>,
    pub resize: Option<String>,
    pub filetype: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Pk,
    Serial,
    Sku,
    Name,
    Title,
    Description,
    Avatar,
    Money,
    Email,
    Phone,
    Address,
    Password,
    Enum(Vec<Symbol>),
    Integer,
    Float,
    Date,
    Timestamp,
    String,
    Text,
    Image,
    File,
    Relation,
    Boolean,
    Uuid,
    // AST-only or unresolved types
    Code,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSchema {
    pub name: Symbol,
    pub entity: Symbol,
    pub field: Symbol,
    pub initial_state: Symbol,
    pub states: Vec<StateSchema>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateSchema {
    pub name: Symbol,
    pub immutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub name: Symbol,
    pub from: Symbol,
    pub to: Symbol,
    pub required_permission: Option<Symbol>,
    #[serde(default)]
    pub preconditions: Vec<TransitionPrecondition>,
    #[serde(default)]
    pub effects: Vec<TransitionEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionPrecondition {
    Assertion(Expression),
    Custom {
        name: Symbol,
        args: Vec<Expression>,
        kwargs: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionEffect {
    Notify(Symbol),
    UpdateField {
        field: Symbol,
        value: String,
    },
    Custom {
        name: Symbol,
        args: Vec<Expression>,
        kwargs: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormSchema {
    pub name: Symbol,
    pub entity: Symbol,
    pub sections: Vec<FormSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormSection {
    pub title: String,
    pub items: Vec<FormItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum FormItem {
    Field(Symbol),
    Grid(GridDef),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridDef {
    pub field: Symbol,
    pub columns: Option<Vec<Symbol>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSchema {
    pub name: Symbol,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSchema {
    pub name: Symbol,
    pub header_enabled: bool,
    pub sidebar_enabled: bool,
    pub footer: Option<String>,
    pub props: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuSchema {
    pub name: Symbol,
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
    pub verb: RouteVerb,
    pub path: String,
    pub action: Symbol,
    pub layout: Option<Symbol>,
    pub permission: Option<Symbol>,
    pub children: Vec<RouteSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum RouteVerb {
    Get,
    Post,
    Put,
    Delete,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSchema {
    pub label: String,
    pub to: Option<Symbol>,
    pub method: Option<RouteVerb>,
    pub icon: Option<String>,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLogic {
    pub name: Symbol,
    pub params: Vec<Symbol>,
    pub steps: Vec<ActionStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStep {
    pub step_type: ActionStepType,
    pub target: Symbol,
    pub args: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionStepType {
    EntityDelete,
    EntityUpdate,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSchema {
    pub name: Symbol,
    pub title: String,
    pub widgets: Vec<WidgetSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetSchema {
    pub name: Symbol,
    pub widget_type: WidgetType,
    pub label: Option<String>,
    pub value: Option<String>,
    pub icon: Option<String>,
    pub roles: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WidgetType {
    Stat,
    Chart,
    List,
    Pie,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialGeneratorSchema {
    pub name: Symbol,
    pub prefix: Option<String>,
    pub date_format: Option<String>,
    pub digits: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintSchema {
    pub name: Symbol,
    pub entity: Symbol,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSchema {
    pub name: Symbol,
    pub title: String,
    pub content: PageContentSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageContentSchema {
    Datatable(DatatableSchema),
    Form(Symbol),
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatatableColumnSchema {
    pub field: Symbol,
    pub label: String,
}

impl Default for Schema {
    fn default() -> Self {
        Self {
            name: Symbol::from(""),
            version: String::default(),
            database: Option::default(),
            storages: HashMap::default(),
            modules: HashMap::default(),
            entities: HashMap::default(),
            tables: HashMap::default(),
            workflows: HashMap::default(),
            forms: HashMap::default(),
            actions: HashMap::default(),
            permissions: HashMap::default(),
            layouts: HashMap::default(),
            menus: HashMap::default(),
            routes: HashMap::default(),
            pages: HashMap::default(),
            dashboards: HashMap::default(),
            serial_generators: HashMap::default(),
            prints: HashMap::default(),
            queries: HashMap::default(),
            rules: HashMap::default(),
            posting_rules: HashMap::default(),
        }
    }
}
