use crate::diagnostics::SourceSpan;

#[derive(Debug, Clone)]
pub struct Ast {
    pub name: Option<String>,
    pub version: Option<String>,
    pub database: Option<DatabaseDef>,
    pub icons: Vec<IconDef>,
    pub layouts: Vec<LayoutDef>,
    pub modules: Vec<ModuleDef>,
    pub entities: Vec<EntityDef>,
    pub tables: Vec<TableDef>, // Added
    pub enums: Vec<EnumDef>,
    pub serial_generators: Vec<SerialGeneratorDef>,
    pub workflows: Vec<WorkflowDef>,
    pub dashboards: Vec<DashboardDef>,
    pub pages: Vec<PageDef>,
    pub actions: Vec<ActionLogicDef>, // Added
    pub routes: Vec<RoutesDef>,
    pub menus: Vec<MenuDef>,
    pub prints: Vec<PrintDef>,
    pub permissions: Vec<PermissionDef>, // Global roles/permissions
}

#[derive(Debug, Clone)]
pub struct DatabaseDef {
    pub db_type: String, // postgres | sqlite
    pub url: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct IconDef {
    pub name: String,
    pub uri: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct LayoutDef {
    pub name: String,
    pub header: Option<LayoutSectionDef>,
    pub sidebar: Option<LayoutSectionDef>,
    pub footer: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct LayoutSectionDef {
    pub enabled: bool,
    pub props: std::collections::HashMap<String, String>, // generic props like search_bar=true
    pub menu_ref: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct ModuleDef {
    pub name: String,
    pub entities: Vec<EntityDef>,
    pub enums: Vec<EnumDef>,
    pub actions: Vec<ActionLogicDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct SerialGeneratorDef {
    pub name: String,
    pub prefix: Option<String>,
    pub date_format: Option<String>,
    pub sequence_digits: u32,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct EntityDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
    pub relationships: Vec<RelationshipDef>,
    pub options: EntityOptions,
    pub seeds: Vec<std::collections::HashMap<String, String>>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Default)]
pub struct EntityOptions {
    pub is_submittable: bool,
    pub track_changes: bool,
    pub is_single: bool,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub type_name: String,                // Semantic Type
    pub serial_generator: Option<String>, // if type is code
    pub required: bool,
    pub unique: bool,
    pub default: Option<String>,
    pub references: Option<String>, // For simple refs or enum links
    pub span: SourceSpan,
}

// Added Table Definitions
#[derive(Debug, Clone)]
pub struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub type_name: String,                                // "serial", "varchar", etc.
    pub props: std::collections::HashMap<String, String>, // len, precision, etc.
    pub primary: bool,
    pub unique: bool,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub enum RelationshipType {
    BelongsTo,
    HasMany,
    HasOne,
}

#[derive(Debug, Clone)]
pub struct RelationshipDef {
    pub rel_type: RelationshipType,
    pub name: String, // field name, e.g. "orders"
    pub target_entity: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct WorkflowDef {
    pub name: String,
    pub entity: String,
    pub field: String,
    pub states: Vec<StateDef>,
    pub transitions: Vec<TransitionDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct StateDef {
    pub name: String,
    pub initial: bool,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct TransitionDef {
    pub name: String,
    pub from: String,
    pub to: String, // could be single or multiple if strictly parsed, but DSL ex suggests single
    pub permission: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct DashboardDef {
    pub name: String,
    pub title: String,
    pub widgets: Vec<WidgetDef>, // Simplified: flatten grid/row for now or recurse
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct WidgetDef {
    pub name: String,
    pub widget_type: String, // stat, chart, list
    pub label: Option<String>,
    pub value: Option<String>, // expression
    pub icon: Option<String>,
    // Add more specific props as map if needed
    pub query: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct PageDef {
    pub name: String,
    pub title: String,
    pub layout: Option<String>,
    pub content: PageContent,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub enum PageContent {
    Datatable(DatatableDef),
    Form(FormDef),
    Dashboard, // Placeholder if page wraps dashboard
    None,
}

#[derive(Debug, Clone)]
pub struct DatatableDef {
    pub entity: String,
    pub columns: Vec<DatatableColumnDef>,
    pub actions: Vec<ActionDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct DatatableColumnDef {
    pub field: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct ActionDef {
    pub label: String,
    pub icon: Option<String>,
    pub to: Option<String>,
    pub method: Option<String>, // Added support for explicit method (DELETE, POST)
    pub variant: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FormDef {
    pub name: String,
    pub entity: String, // can be anonymous if embedded
    pub sections: Vec<FormSectionDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct FormSectionDef {
    pub title: String,
    pub fields: Vec<String>, // simple field names
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct ActionLogicDef {
    pub name: String,
    pub params: Vec<String>,
    pub steps: Vec<ActionStepDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct ActionStepDef {
    pub step_type: String,                               // entity:delete, entity:update, etc.
    pub target: String,                                  // Entity Name or variable
    pub args: std::collections::HashMap<String, String>, // id=param("id")
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct RoutesDef {
    pub routes: Vec<RouteNode>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub enum RouteNode {
    Route(RouteDef),
    Group(RouteGroupDef),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RouteVerb {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Debug, Clone)]
pub struct RouteDef {
    pub verb: RouteVerb,
    pub path: String,
    pub action: String,         // Action Name (e.g. "DeletePosition")
    pub layout: Option<String>, // Maybe not needed for API routes but kept for legacy pages
    pub permission: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct RouteGroupDef {
    pub path: String,
    pub layout: Option<String>,
    pub permission: Option<String>,
    pub children: Vec<RouteNode>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct MenuDef {
    pub name: String,
    pub items: Vec<MenuItemDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub enum MenuItemDef {
    Item(MenuItem),
    Group(MenuGroup),
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub to: String,
    pub icon: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct MenuGroup {
    pub label: String,
    pub icon: Option<String>,
    pub children: Vec<MenuItemDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct PrintDef {
    pub name: String,
    pub entity: String,
    pub title: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct PermissionDef {
    pub name: String,
    pub allows: Vec<AllowDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct AllowDef {
    pub resource: String,        // url or permission key like sales.*
    pub actions: Option<String>, // read,write
}
