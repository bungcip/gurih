use miette::SourceSpan;

#[derive(Debug, Clone)]
pub struct Ast {
    pub name: Option<String>,
    pub version: Option<String>,
    pub modules: Vec<ModuleDef>,
    pub entities: Vec<EntityDef>,
    pub workflows: Vec<WorkflowDef>,
    pub forms: Vec<FormDef>,
    pub permissions: Vec<PermissionDef>,
}

#[derive(Debug, Clone)]
pub struct ModuleDef {
    pub name: String,
    pub entities: Vec<EntityDef>, // Nested entities
    // We could nesting other things too, but for now just entities as per kdl
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct EntityDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub type_name: String,
    pub required: bool,
    pub unique: bool,
    pub references: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct WorkflowDef {
    pub name: String,
    pub entity: String,
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
    pub to: String,
    pub permission: Option<String>,
    pub span: SourceSpan,
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
pub struct PermissionDef {
    pub name: String,
    pub rules: Vec<String>,
    pub span: SourceSpan,
}
