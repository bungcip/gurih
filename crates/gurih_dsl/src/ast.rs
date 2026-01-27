use crate::diagnostics::SourceSpan;
pub use gurih_ir::{
    ActionStepType, DatabaseType, FieldType, QueryType, RelationshipType, RouteVerb, StorageDriver, WidgetType,
};

#[derive(Debug, Clone)]
pub struct Ast {
    pub name: Option<String>,
    pub version: Option<String>,
    pub database: Option<DatabaseDef>,
    pub storages: Vec<StorageDef>,
    pub icons: Vec<IconDef>,
    pub layouts: Vec<LayoutDef>,
    pub modules: Vec<ModuleDef>,
    pub entities: Vec<EntityDef>,
    pub tables: Vec<TableDef>,
    pub enums: Vec<EnumDef>,
    pub serial_generators: Vec<SerialGeneratorDef>,
    pub workflows: Vec<WorkflowDef>,
    pub dashboards: Vec<DashboardDef>,
    pub pages: Vec<PageDef>,
    pub actions: Vec<ActionLogicDef>,
    pub routes: Vec<RoutesDef>,
    pub menus: Vec<MenuDef>,
    pub prints: Vec<PrintDef>,
    pub queries: Vec<QueryDef>,
    pub permissions: Vec<PermissionDef>,
    pub employee_statuses: Vec<EmployeeStatusDef>,
    pub accounts: Vec<AccountDef>,
    pub rules: Vec<RuleDef>,
    pub posting_rules: Vec<PostingRuleDef>,
}

#[derive(Debug, Clone)]
pub struct PostingRuleDef {
    pub name: String,
    pub source_entity: String,
    pub description_expr: String,
    pub date_expr: String,
    pub lines: Vec<PostingLineDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct PostingLineDef {
    pub account: String,
    pub debit_expr: Option<String>,
    pub credit_expr: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct RuleDef {
    pub name: String,
    pub on_event: String,
    pub assertion: String,
    pub message: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct AccountDef {
    pub name: String,
    pub fields: std::collections::HashMap<String, String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct DatabaseDef {
    pub db_type: DatabaseType,
    pub url: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct StorageDef {
    pub name: String,
    pub driver: StorageDriver,
    pub location: Option<String>,
    pub props: std::collections::HashMap<String, String>,
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
    pub props: std::collections::HashMap<String, String>,
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
    pub type_name: FieldType,
    pub serial_generator: Option<String>,
    pub required: bool,
    pub unique: bool,
    pub default: Option<String>,
    pub references: Option<String>,
    pub storage: Option<String>,
    pub resize: Option<String>,
    pub filetype: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub type_name: String, // Keeping as String for raw SQL types unless we want to enumerate them
    pub props: std::collections::HashMap<String, String>,
    pub primary: bool,
    pub unique: bool,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct RelationshipDef {
    pub rel_type: RelationshipType,
    pub name: String,
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
    pub immutable: bool,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct TransitionDef {
    pub name: String,
    pub from: String,
    pub to: String,
    pub permission: Option<String>,
    pub preconditions: Vec<TransitionPreconditionDef>,
    pub effects: Vec<TransitionEffectDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub enum TransitionPreconditionDef {
    Document {
        name: String,
        span: SourceSpan,
    },
    MinYearsOfService {
        years: u32,
        from_field: Option<String>,
        span: SourceSpan,
    },
    MinAge {
        age: u32,
        birth_date_field: Option<String>,
        span: SourceSpan,
    },
    ValidEffectiveDate {
        field: String,
        span: SourceSpan,
    },
    BalancedTransaction {
        span: SourceSpan,
    },
    PeriodOpen {
        entity: Option<String>,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone)]
pub enum TransitionEffectDef {
    SuspendPayroll {
        active: bool,
        span: SourceSpan,
    },
    Notify {
        target: String,
        span: SourceSpan,
    },
    UpdateRankEligibility {
        active: bool,
        span: SourceSpan,
    },
    UpdateField {
        field: String,
        value: String,
        span: SourceSpan,
    },
    PostJournal {
        rule: String,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone)]
pub struct DashboardDef {
    pub name: String,
    pub title: String,
    pub widgets: Vec<WidgetDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct WidgetDef {
    pub name: String,
    pub widget_type: WidgetType,
    pub label: Option<String>,
    pub value: Option<String>,
    pub icon: Option<String>,
    pub query: Option<String>,
    pub roles: Option<Vec<String>>,
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
    Dashboard,
    None,
}

#[derive(Debug, Clone)]
pub struct QueryDef {
    pub name: String,
    pub root_entity: String,
    pub query_type: QueryType,
    pub selections: Vec<QuerySelectionDef>,
    pub formulas: Vec<QueryFormulaDef>,
    pub filters: Vec<String>,
    pub joins: Vec<QueryJoinDef>,
    pub group_by: Vec<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct QuerySelectionDef {
    pub field: String,
    pub alias: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct QueryFormulaDef {
    pub name: String,
    pub expression: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct QueryJoinDef {
    pub target_entity: String,
    pub selections: Vec<QuerySelectionDef>,
    pub formulas: Vec<QueryFormulaDef>,
    pub joins: Vec<QueryJoinDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct DatatableDef {
    pub entity: Option<String>,
    pub query: Option<String>,
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
    pub method: Option<RouteVerb>,
    pub variant: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FormDef {
    pub name: String,
    pub entity: String,
    pub sections: Vec<FormSectionDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct FormSectionDef {
    pub title: String,
    pub fields: Vec<String>,
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
    pub step_type: ActionStepType,
    pub target: String,
    pub args: std::collections::HashMap<String, String>,
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

#[derive(Debug, Clone)]
pub struct RouteDef {
    pub verb: RouteVerb,
    pub path: String,
    pub action: String,
    pub layout: Option<String>,
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
    pub resource: String,
    pub actions: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EmployeeStatusDef {
    pub name: String,
    pub entity: Option<String>,
    pub field: Option<String>,
    pub transitions: Vec<TransitionDef>,
    pub span: SourceSpan,
}
