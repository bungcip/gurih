use crate::ast::*;
use crate::diagnostics::SourceSpan;
use crate::errors::CompileError;
use crate::expr::parse_expression;
use crate::utils::*;
use kdl::{KdlDocument, KdlNode};
use std::collections::HashMap;
use std::path::Path;

pub fn parse(src: &str, base_path: Option<&Path>) -> Result<Ast, CompileError> {
    let doc: KdlDocument = src.parse()?;
    let mut ast = Ast {
        name: None,
        version: None,
        database: None,
        icons: vec![],
        layouts: vec![],
        modules: vec![],
        entities: vec![],
        tables: vec![],
        enums: vec![],
        serial_generators: vec![],
        workflows: vec![],
        dashboards: vec![],
        pages: vec![],
        routes: vec![],
        menus: vec![],
        prints: vec![],
        permissions: vec![],
        actions: vec![],
        storages: vec![],
        queries: vec![],
        accounts: vec![],
        rules: vec![],
        posting_rules: vec![],
        employee_statuses: vec![],
    };

    for node in doc.nodes() {
        let name = node.name().value();
        match name {
            "name" => ast.name = Some(get_arg_string(node, 0, src)?),
            "version" => ast.version = Some(get_arg_string(node, 0, src)?),
            "storage" => ast.storages.push(parse_storage(node, src)?),
            "include" => {
                if let Some(base) = base_path {
                    let filename = get_arg_string(node, 0, src)?;
                    let path = base.join(&filename);
                    let content = std::fs::read_to_string(&path).map_err(|e| CompileError::ParseError {
                        src: src.to_string(),
                        span: node.name().span().into(),
                        message: format!("Failed to read include file {}: {}", path.display(), e),
                    })?;
                    // Included files are relative to their own location
                    let new_base = path.parent();
                    let included_ast = parse(&content, new_base)?;

                    // Merge included AST
                    if included_ast.name.is_some() && ast.name.is_none() {
                        ast.name = included_ast.name;
                    }
                    if included_ast.version.is_some() && ast.version.is_none() {
                        ast.version = included_ast.version;
                    }
                    if included_ast.database.is_some() && ast.database.is_none() {
                        ast.database = included_ast.database;
                    }
                    ast.icons.extend(included_ast.icons);
                    ast.layouts.extend(included_ast.layouts);
                    ast.modules.extend(included_ast.modules);
                    ast.entities.extend(included_ast.entities);
                    ast.tables.extend(included_ast.tables);
                    ast.enums.extend(included_ast.enums);
                    ast.serial_generators.extend(included_ast.serial_generators);
                    ast.workflows.extend(included_ast.workflows);
                    ast.dashboards.extend(included_ast.dashboards);
                    ast.pages.extend(included_ast.pages);
                    ast.actions.extend(included_ast.actions);
                    ast.routes.extend(included_ast.routes);
                    ast.menus.extend(included_ast.menus);
                    ast.prints.extend(included_ast.prints);
                    ast.permissions.extend(included_ast.permissions);
                    ast.accounts.extend(included_ast.accounts);
                    ast.rules.extend(included_ast.rules);
                    ast.posting_rules.extend(included_ast.posting_rules);
                    ast.queries.extend(included_ast.queries);
                    ast.employee_statuses.extend(included_ast.employee_statuses);
                } else {
                    return Err(CompileError::ParseError {
                        src: src.to_string(),
                        span: node.name().span().into(),
                        message: "Includes are not supported in this context (missing base path)".to_string(),
                    });
                }
            }
            "database" | "persistence" | "datastore" => ast.database = Some(parse_database(node, src)?),
            "icons" => ast.icons.extend(parse_icons(node, src)?),
            "layout" => ast.layouts.push(parse_layout(node, src)?),
            "module" => ast.modules.push(parse_module(node, src)?),
            "entity" => ast.entities.push(parse_entity(node, src)?),
            "table" => ast.tables.push(parse_table(node, src)?),
            "enum" => ast.enums.push(parse_enum(node, src)?),
            "serial_generator" => ast.serial_generators.push(parse_serial_generator(node, src)?),
            "workflow" => ast.workflows.push(parse_workflow(node, src)?),
            "dashboard" => ast.dashboards.push(parse_dashboard(node, src)?),
            "page" => ast.pages.push(parse_page(node, src)?),
            "action" => ast.actions.push(parse_action_logic(node, src)?),
            "routes" => ast.routes.push(parse_routes(node, src)?),
            "menu" => ast.menus.push(parse_menu(node, src)?),
            "print" => ast.prints.push(parse_print(node, src)?),
            "role" | "permission" => ast.permissions.push(parse_permission(node, src)?),
            "query" | "query:nested" | "query:flat" | "query:hierarchy" => ast.queries.push(parse_query(node, src)?),
            "account" => ast.accounts.push(parse_account(node, src)?),
            "rule" => ast.rules.push(parse_rule(node, src)?),
            "posting_rule" => ast.posting_rules.push(parse_posting_rule(node, src)?),
            "employee_status" => ast.employee_statuses.push(parse_employee_status(node, src)?),
            _ => {
                return Err(CompileError::ParseError {
                    src: src.to_string(),
                    span: node.name().span().into(),
                    message: format!("Unknown top-level definition: {}", name),
                });
            }
        }
    }

    Ok(ast)
}

#[allow(clippy::collapsible_if)]
fn parse_action_logic(node: &KdlNode, src: &str) -> Result<ActionLogicDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut params = vec![];
    let mut steps = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "param" => params.push(get_arg_string(child, 0, src)?),
                "step" => {
                    let step_type_str = get_arg_string(child, 0, src)?;
                    let span = child.span();
                    let step_type = parse_step_type(&step_type_str, span.into(), src)?;

                    let target = get_prop_string(child, "target", src).unwrap_or_default();

                    let mut args = std::collections::HashMap::new();
                    for entry in child.entries() {
                        if let Some(key) = entry.name() {
                            let key_str = key.value();
                            if key_str != "target" {
                                if let Some(val) = entry.value().as_string() {
                                    args.insert(key_str.to_string(), val.to_string());
                                }
                            }
                        }
                    }
                    steps.push(ActionStepDef {
                        step_type,
                        target,
                        args,
                        span: span.into(),
                    });
                }
                step_type_str if step_type_str.starts_with("step:") => {
                    let target = get_arg_string(child, 0, src)?;
                    let mut args = std::collections::HashMap::new();
                    for entry in child.entries() {
                        if let Some(key) = entry.name() {
                            if let Some(val) = entry.value().as_string() {
                                args.insert(key.value().to_string(), val.to_string());
                            }
                        }
                    }
                    let span = child.span();
                    steps.push(ActionStepDef {
                        step_type: parse_step_type(
                            step_type_str.strip_prefix("step:").unwrap_or(step_type_str),
                            span.into(),
                            src,
                        )?,
                        target,
                        args,
                        span: span.into(),
                    });
                }
                _ => {}
            }
        }
    }

    Ok(ActionLogicDef {
        name,
        params,
        steps,
        span: node.span().into(),
    })
}

fn parse_employee_status(node: &KdlNode, src: &str) -> Result<EmployeeStatusDef, CompileError> {
    let status = get_arg_string(node, 0, src)?;
    let entity = get_prop_string(node, "for", src)
        .or_else(|_| get_prop_string(node, "entity", src))
        .unwrap_or_else(|_| "Pegawai".to_string());
    let field = get_prop_string(node, "field", src).ok();
    let initial = get_prop_bool(node, "initial").unwrap_or(false);

    let mut transitions = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            if child.name().value() == "can_transition_to" {
                transitions.push(parse_status_transition(child, src, &status)?);
            }
        }
    }

    Ok(EmployeeStatusDef {
        status,
        entity,
        field,
        initial,
        transitions,
        span: node.span().into(),
    })
}

fn parse_status_transition(node: &KdlNode, src: &str, from_status: &str) -> Result<TransitionDef, CompileError> {
    let target = get_arg_string(node, 0, src)?;
    let permission = get_prop_string(node, "permission", src).ok();

    let (preconditions, effects) = if let Some(children) = node.children() {
        parse_transition_body(children, src)?
    } else {
        (vec![], vec![])
    };

    let name = format!("{}_to_{}", from_status, target);

    Ok(TransitionDef {
        name,
        from: from_status.to_string(),
        to: target,
        permission,
        preconditions,
        effects,
        span: node.span().into(),
    })
}

fn parse_step_type(s: &str, _span: SourceSpan, _src: &str) -> Result<ActionStepType, CompileError> {
    match s {
        "entity:delete" => Ok(ActionStepType::EntityDelete),
        "entity:update" => Ok(ActionStepType::EntityUpdate),
        "entity:create" => Ok(ActionStepType::EntityCreate),
        custom => Ok(ActionStepType::Custom(custom.to_string())),
    }
}

fn parse_database(node: &KdlNode, src: &str) -> Result<DatabaseDef, CompileError> {
    let mut db_type = DatabaseType::Postgres;
    let mut url = String::new();

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "type" => {
                    let t = get_arg_string(child, 0, src)?;
                    db_type = match t.to_lowercase().as_str() {
                        "postgres" => DatabaseType::Postgres,
                        "sqlite" => DatabaseType::Sqlite,
                        _ => {
                            return Err(CompileError::ParseError {
                                src: src.to_string(),
                                span: child.span().into(),
                                message: format!("Unsupported database type: {}", t),
                            });
                        }
                    };
                }
                "url" => url = get_arg_string(child, 0, src)?,
                _ => {}
            }
        }
    }

    Ok(DatabaseDef {
        db_type,
        url,
        span: node.span().into(),
    })
}

fn parse_icons(node: &KdlNode, src: &str) -> Result<Vec<IconDef>, CompileError> {
    let mut icons = vec![];
    if let Some(children) = node.children() {
        for child in children.nodes() {
            let name = child.name().value().to_string();
            let uri = get_arg_string(child, 0, src)?;
            icons.push(IconDef {
                name,
                uri,
                span: child.span().into(),
            });
        }
    }
    Ok(icons)
}

fn parse_layout(node: &KdlNode, src: &str) -> Result<LayoutDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut header = None;
    let mut sidebar = None;
    let mut footer = None;

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "header" => header = Some(parse_layout_section(child, src)?),
                "sidebar" => sidebar = Some(parse_layout_section(child, src)?),
                "footer" => footer = Some(get_arg_string(child, 0, src)?),
                _ => {}
            }
        }
    }

    Ok(LayoutDef {
        name,
        header,
        sidebar,
        footer,
        span: node.span().into(),
    })
}

fn parse_layout_section(node: &KdlNode, src: &str) -> Result<LayoutSectionDef, CompileError> {
    let enabled = get_arg_bool(node, 0).unwrap_or(true);

    let mut props = std::collections::HashMap::new();
    let mut menu_ref = None;

    if enabled && let Some(children) = node.children() {
        for child in children.nodes() {
            let key = child.name().value();
            if key == "menu_ref" {
                menu_ref = Some(get_arg_string(child, 0, src)?);
            } else if let Ok(val) = get_arg_string(child, 0, src) {
                props.insert(key.to_string(), val);
            } else if let Ok(val) = get_arg_bool(child, 0) {
                props.insert(key.to_string(), val.to_string());
            }
        }
    }

    Ok(LayoutSectionDef {
        enabled,
        props,
        menu_ref,
        span: node.span().into(),
    })
}

fn parse_module(node: &KdlNode, src: &str) -> Result<ModuleDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut entities = vec![];
    let mut actions = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "entity" => entities.push(parse_entity(child, src)?),
                "action" => actions.push(parse_action_logic(child, src)?),
                _ => {}
            }
        }
    }

    Ok(ModuleDef {
        name,
        entities,
        enums: vec![],
        actions,
        span: node.span().into(),
    })
}

fn parse_entity(node: &KdlNode, src: &str) -> Result<EntityDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut fields = vec![];
    let mut relationships = vec![];
    let mut options = EntityOptions::default();
    let mut seeds = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            let child_name = child.name().value();
            match child_name {
                "field" => fields.push(parse_field(child, src)?),
                name if name.starts_with("field:") => {
                    let type_part = &name[6..];
                    let mut field_def = parse_field(child, src)?;
                    field_def.type_name = parse_field_type_str(type_part);
                    fields.push(field_def);
                }
                "options" => {
                    if let Some(opts) = child.children() {
                        for opt in opts.nodes() {
                            match opt.name().value() {
                                "is_submittable" => options.is_submittable = get_arg_bool(opt, 0).unwrap_or(false),
                                "track_changes" => options.track_changes = get_arg_bool(opt, 0).unwrap_or(false),
                                "is_single" => options.is_single = get_arg_bool(opt, 0).unwrap_or(false),
                                _ => {}
                            }
                        }
                    }
                }
                "seed" => {
                    if let Some(rows) = child.children() {
                        for row_node in rows.nodes() {
                            if row_node.name().value() == "row" || row_node.name().value() == "data" {
                                let mut row_data = HashMap::new();
                                for entry in row_node.entries() {
                                    if let Some(k) = entry.name() {
                                        let key_str = k.value();
                                        if let Some(val) = entry.value().as_string() {
                                            row_data.insert(key_str.to_string(), val.to_string());
                                        } else if let Some(val) = entry.value().as_integer() {
                                            row_data.insert(key_str.to_string(), val.to_string());
                                        } else if let Some(val) = entry.value().as_bool() {
                                            row_data.insert(key_str.to_string(), val.to_string());
                                        } else if let Some(val) = entry.value().as_float() {
                                            row_data.insert(key_str.to_string(), val.to_string());
                                        }
                                    }
                                }
                                seeds.push(row_data);
                            }
                        }
                    }
                }
                "belongs_to" | "has_many" | "has_one" => {
                    let rel_type = match child_name {
                        "belongs_to" => RelationshipType::BelongsTo,
                        "has_many" => RelationshipType::HasMany,
                        "has_one" => RelationshipType::HasOne,
                        _ => unreachable!(),
                    };

                    let arg0 = get_arg_string(child, 0, src)?;
                    let (name, target) = if let Ok(arg1) = get_arg_string(child, 1, src) {
                        (arg0, arg1)
                    } else {
                        let name = arg0.to_lowercase();
                        (name, arg0)
                    };

                    let ownership_str = get_prop_string(child, "type", src).ok();
                    let ownership = match ownership_str.as_deref() {
                        Some("composition") => Ownership::Composition,
                        _ => Ownership::Reference,
                    };

                    relationships.push(RelationshipDef {
                        rel_type,
                        name,
                        target_entity: target,
                        ownership,
                        span: child.span().into(),
                    });
                }
                "id" => {
                    fields.push(FieldDef {
                        name: "id".to_string(),
                        type_name: FieldType::Integer,
                        serial_generator: None,
                        required: true,
                        unique: true,
                        default: None,
                        references: None,
                        storage: None,
                        resize: None,
                        filetype: None,
                        span: child.span().into(),
                    });
                }
                "string" | "text" | "int" | "integer" | "float" | "decimal" | "bool" | "boolean" | "date"
                | "datetime" | "timestamp" | "time" | "money" | "code" | "enum" | "name" | "email" | "phone"
                | "description" | "pk" | "serial" | "sku" | "title" | "avatar" | "address" | "image" | "file" => {
                    let type_name = parse_field_type_str(child_name);
                    let field_name = get_arg_string(child, 0, src).unwrap_or(child_name.to_string());
                    fields.push(parse_field_props(child, src, field_name, type_name)?);
                }
                _ => {}
            }
        }
    }

    Ok(EntityDef {
        name,
        fields,
        relationships,
        options,
        seeds,
        span: node.span().into(),
    })
}

fn parse_table(node: &KdlNode, src: &str) -> Result<TableDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut columns = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            if child.name().value() == "column" {
                columns.push(parse_column(child, src)?);
            }
        }
    }

    Ok(TableDef {
        name,
        columns,
        span: node.span().into(),
    })
}

fn parse_column(node: &KdlNode, src: &str) -> Result<ColumnDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let type_name = get_prop_string(node, "type", src)?;
    let primary = get_prop_bool(node, "primary").unwrap_or(false);
    let unique = get_prop_bool(node, "unique").unwrap_or(false);

    let mut props = std::collections::HashMap::new();
    for entry in node.entries() {
        if let Some(key) = entry.name() {
            let k = key.value();
            if !k.is_empty() && k != "type" && k != "primary" && k != "unique" {
                if let Some(s) = entry.value().as_string() {
                    props.insert(k.to_string(), s.to_string());
                } else if let Some(i) = entry.value().as_integer() {
                    props.insert(k.to_string(), i.to_string());
                }
            }
        }
    }

    Ok(ColumnDef {
        name,
        type_name,
        props,
        primary,
        unique,
        span: node.span().into(),
    })
}

fn parse_field(node: &KdlNode, src: &str) -> Result<FieldDef, CompileError> {
    let name = get_arg_string(node, 0, src).unwrap_or_else(|_| "unknown".to_string());
    let type_str = get_prop_string(node, "type", src).unwrap_or_else(|_| "String".to_string());
    let type_name = parse_field_type_str(&type_str);
    parse_field_props(node, src, name, type_name)
}

fn parse_field_props(node: &KdlNode, src: &str, name: String, type_name: FieldType) -> Result<FieldDef, CompileError> {
    let node_name = node.name().value();

    let references = if node_name == "enum" || node_name == "field:enum" {
        get_arg_string(node, 1, src)
            .ok()
            .or_else(|| get_prop_string(node, "references", src).ok())
    } else {
        get_prop_string(node, "references", src).ok()
    };

    let required_prop = get_prop_bool(node, "required");
    let nullable_prop = get_prop_bool(node, "nullable");

    let required = if let Some(n) = nullable_prop {
        !n
    } else {
        required_prop.unwrap_or(false)
    };

    let unique = get_prop_bool(node, "unique").unwrap_or(false);
    let default = get_prop_string(node, "default", src).ok();
    let serial_generator = get_prop_string(node, "serial_generator", src).ok();

    let storage = get_prop_string(node, "storage", src).ok();
    let resize = get_prop_string(node, "resize", src).ok();
    let filetype = get_prop_string(node, "filetype", src).ok();

    Ok(FieldDef {
        name,
        type_name,
        serial_generator,
        required,
        unique,
        default,
        references,
        storage,
        resize,
        filetype,
        span: node.span().into(),
    })
}

fn parse_field_type_str(s: &str) -> FieldType {
    match s.to_lowercase().as_str() {
        "pk" => FieldType::Pk,
        "serial" => FieldType::Serial,
        "sku" => FieldType::Sku,
        "name" => FieldType::Name,
        "title" => FieldType::Title,
        "description" => FieldType::Description,
        "avatar" => FieldType::Avatar,
        "money" => FieldType::Money,
        "email" => FieldType::Email,
        "phone" => FieldType::Phone,
        "address" => FieldType::Address,
        "password" => FieldType::Password,
        "enum" => FieldType::Enum(vec![]),
        "int" | "integer" => FieldType::Integer,
        "float" | "decimal" => FieldType::Float,
        "date" => FieldType::Date,
        "datetime" | "timestamp" => FieldType::Timestamp,
        "string" => FieldType::String,
        "text" => FieldType::Text,
        "uuid" => FieldType::Uuid,
        "image" | "photo" => FieldType::Image,
        "file" => FieldType::File,
        "relation" => FieldType::Relation,
        "bool" | "boolean" => FieldType::Boolean,
        "code" => FieldType::Code,
        _ => FieldType::Custom(capitalize(s)),
    }
}

fn parse_storage(node: &KdlNode, src: &str) -> Result<StorageDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut driver = StorageDriver::Local;
    let mut location = None;
    let mut props = std::collections::HashMap::new();

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "driver" => {
                    let d = get_arg_string(child, 0, src)?;
                    driver = match d.to_lowercase().as_str() {
                        "s3" => StorageDriver::S3,
                        "file" | "local" => StorageDriver::Local,
                        _ => {
                            return Err(CompileError::ParseError {
                                src: src.to_string(),
                                span: child.span().into(),
                                message: format!("Unsupported storage driver: {}", d),
                            });
                        }
                    };
                }
                "location" => location = Some(get_arg_string(child, 0, src)?),
                key => {
                    if let Ok(val) = get_arg_string(child, 0, src) {
                        props.insert(key.to_string(), val);
                    }
                }
            }
        }
    }

    Ok(StorageDef {
        name,
        driver,
        location,
        props,
        span: node.span().into(),
    })
}

fn parse_workflow(node: &KdlNode, src: &str) -> Result<WorkflowDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let entity = get_prop_string(node, "for", src).or_else(|_| get_prop_string(node, "entity", src))?;

    let field = get_prop_string(node, "field", src)?;

    let mut states = vec![];
    let mut transitions = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "state" => states.push(parse_state(child, src)?),
                "transition" => transitions.push(parse_transition(child, src)?),
                _ => {}
            }
        }
    }

    Ok(WorkflowDef {
        name,
        entity,
        field,
        states,
        transitions,
        span: node.span().into(),
    })
}

fn parse_state(node: &KdlNode, src: &str) -> Result<StateDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let initial = get_prop_bool(node, "initial").unwrap_or(false);
    let immutable = get_prop_bool(node, "immutable").unwrap_or(false);

    Ok(StateDef {
        name,
        initial,
        immutable,
        span: node.span().into(),
    })
}

fn parse_transition_body(
    children: &KdlDocument,
    src: &str,
) -> Result<(Vec<TransitionPreconditionDef>, Vec<TransitionEffectDef>), CompileError> {
    let mut preconditions = vec![];
    let mut effects = vec![];

    for child in children.nodes() {
        match child.name().value() {
            "requires" => {
                if let Some(req_children) = child.children() {
                    for req in req_children.nodes() {
                        match req.name().value() {
                            "document" => {
                                let doc_name = get_arg_string(req, 0, src)?;
                                let span = req
                                    .entries()
                                    .first()
                                    .map(|e| e.span().offset())
                                    .unwrap_or(req.span().offset());
                                let expr_str = format!("is_set({})", doc_name);
                                let expr = parse_expression(&expr_str, span)?;
                                preconditions.push(TransitionPreconditionDef::Assertion {
                                    expression: expr,
                                    span: req.span().into(),
                                });
                            }
                            custom_req => {
                                let mut args = vec![];
                                let mut kwargs = HashMap::new();
                                for entry in req.entries() {
                                    if let Some(key) = entry.name() {
                                        if let Some(val) = entry.value().as_string() {
                                            kwargs.insert(key.value().to_string(), val.to_string());
                                        } else if let Some(val) = entry.value().as_bool() {
                                            kwargs.insert(key.value().to_string(), val.to_string());
                                        } else if let Some(val) = entry.value().as_integer() {
                                            kwargs.insert(key.value().to_string(), val.to_string());
                                        } else if let Some(val) = entry.value().as_float() {
                                            kwargs.insert(key.value().to_string(), val.to_string());
                                        }
                                    } else if let Some(val) = entry.value().as_string() {
                                        args.push(val.to_string());
                                    } else if let Some(val) = entry.value().as_bool() {
                                        args.push(val.to_string());
                                    } else if let Some(val) = entry.value().as_integer() {
                                        args.push(val.to_string());
                                    } else if let Some(val) = entry.value().as_float() {
                                        args.push(val.to_string());
                                    }
                                }
                                preconditions.push(TransitionPreconditionDef::Custom {
                                    name: custom_req.to_string(),
                                    args,
                                    kwargs,
                                    span: req.span().into(),
                                });
                            }
                        }
                    }
                }
            }
            "effects" => {
                if let Some(eff_children) = child.children() {
                    for eff in eff_children.nodes() {
                        match eff.name().value() {
                            "notify" => {
                                let target = get_arg_string(eff, 0, src)?;
                                effects.push(TransitionEffectDef::Notify {
                                    target,
                                    span: eff.span().into(),
                                });
                            }
                            "update" => {
                                let field = get_arg_string(eff, 0, src)?;
                                let value = get_arg_string(eff, 1, src)?;
                                effects.push(TransitionEffectDef::UpdateField {
                                    field,
                                    value,
                                    span: eff.span().into(),
                                });
                            }
                            custom_eff => {
                                let mut args = vec![];
                                let mut kwargs = HashMap::new();
                                for entry in eff.entries() {
                                    if let Some(key) = entry.name() {
                                        if let Some(val) = entry.value().as_string() {
                                            kwargs.insert(key.value().to_string(), val.to_string());
                                        } else if let Some(val) = entry.value().as_bool() {
                                            kwargs.insert(key.value().to_string(), val.to_string());
                                        } else if let Some(val) = entry.value().as_integer() {
                                            kwargs.insert(key.value().to_string(), val.to_string());
                                        } else if let Some(val) = entry.value().as_float() {
                                            kwargs.insert(key.value().to_string(), val.to_string());
                                        }
                                    } else if let Some(val) = entry.value().as_string() {
                                        args.push(val.to_string());
                                    } else if let Some(val) = entry.value().as_bool() {
                                        args.push(val.to_string());
                                    } else if let Some(val) = entry.value().as_integer() {
                                        args.push(val.to_string());
                                    } else if let Some(val) = entry.value().as_float() {
                                        args.push(val.to_string());
                                    }
                                }
                                effects.push(TransitionEffectDef::Custom {
                                    name: custom_eff.to_string(),
                                    args,
                                    kwargs,
                                    span: eff.span().into(),
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok((preconditions, effects))
}

fn parse_transition(node: &KdlNode, src: &str) -> Result<TransitionDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut from = get_prop_string(node, "from", src).ok();
    let mut to = get_prop_string(node, "to", src).ok();
    let permission = get_prop_string(node, "permission", src).ok();

    let mut preconditions = vec![];
    let mut effects = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "from" => from = Some(get_arg_string(child, 0, src)?),
                "to" => to = Some(get_arg_string(child, 0, src)?),
                _ => {}
            }
        }
        let (mut p, mut e) = parse_transition_body(children, src)?;
        preconditions.append(&mut p);
        effects.append(&mut e);
    }

    let from = from.ok_or_else(|| CompileError::ParseError {
        src: src.to_string(),
        span: node.span().into(),
        message: "Missing property 'from'".to_string(),
    })?;

    let to = to.ok_or_else(|| CompileError::ParseError {
        src: src.to_string(),
        span: node.span().into(),
        message: "Missing property 'to'".to_string(),
    })?;

    Ok(TransitionDef {
        name,
        from,
        to,
        permission,
        preconditions,
        effects,
        span: node.span().into(),
    })
}

fn parse_account(node: &KdlNode, src: &str) -> Result<AccountDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut fields = HashMap::new();

    if let Some(children) = node.children() {
        for child in children.nodes() {
            let key = child.name().value();
            if let Ok(val) = get_arg_string(child, 0, src) {
                fields.insert(key.to_string(), val);
            }
        }
    }

    Ok(AccountDef {
        name,
        fields,
        span: node.span().into(),
    })
}

fn parse_rule(node: &KdlNode, src: &str) -> Result<RuleDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut on_event = String::new();
    let mut assertion = None;
    let mut message = String::new();

    if let Some(children) = node.children() {
        for child in children.nodes() {
            let child_name = child.name().value();
            if child_name == "assert" {
                let s = get_arg_string(child, 0, src)?;
                let offset = child
                    .entries()
                    .first()
                    .map(|e| e.span().offset())
                    .unwrap_or(child.span().offset());
                assertion = Some(parse_expression(&s, offset)?);
            } else if child_name == "message" {
                message = get_arg_string(child, 0, src)?;
            } else if let Some(lifecycle) = child_name.strip_prefix("on:") {
                let entity = get_arg_string(child, 0, src)?;
                on_event = format!("{}:{}", entity, lifecycle);
            }
        }
    }

    let assertion = assertion.ok_or_else(|| CompileError::ParseError {
        src: src.to_string(),
        span: node.span().into(),
        message: "Missing assertion in rule".to_string(),
    })?;

    Ok(RuleDef {
        name,
        on_event,
        assertion,
        message,
        span: node.span().into(),
    })
}

fn parse_posting_rule(node: &KdlNode, src: &str) -> Result<PostingRuleDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let source_entity = get_prop_string(node, "for", src)?;
    let mut description_expr = None;
    let mut date_expr = None;
    let mut lines = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "description" => {
                    let s = get_arg_string(child, 0, src)?;
                    let offset = child
                        .entries()
                        .first()
                        .map(|e| e.span().offset())
                        .unwrap_or(child.span().offset());
                    description_expr = Some(parse_expression(&s, offset)?);
                }
                "date" => {
                    let s = get_arg_string(child, 0, src)?;
                    let offset = child
                        .entries()
                        .first()
                        .map(|e| e.span().offset())
                        .unwrap_or(child.span().offset());
                    date_expr = Some(parse_expression(&s, offset)?);
                }
                "entry" | "line" => lines.push(parse_posting_line(child, src)?),
                _ => {}
            }
        }
    }

    let description_expr = description_expr.ok_or_else(|| CompileError::ParseError {
        src: src.to_string(),
        span: node.span().into(),
        message: "Missing description".to_string(),
    })?;
    let date_expr = date_expr.ok_or_else(|| CompileError::ParseError {
        src: src.to_string(),
        span: node.span().into(),
        message: "Missing date".to_string(),
    })?;

    Ok(PostingRuleDef {
        name,
        source_entity,
        description_expr,
        date_expr,
        lines,
        span: node.span().into(),
    })
}

fn parse_posting_line(node: &KdlNode, src: &str) -> Result<PostingLineDef, CompileError> {
    let mut account = String::new();
    let mut debit_expr = None;
    let mut credit_expr = None;

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "account" => account = get_arg_string(child, 0, src)?,
                "debit" => {
                    let s = get_arg_string(child, 0, src)?;
                    let offset = child
                        .entries()
                        .first()
                        .map(|e| e.span().offset())
                        .unwrap_or(child.span().offset());
                    debit_expr = Some(parse_expression(&s, offset)?);
                }
                "credit" => {
                    let s = get_arg_string(child, 0, src)?;
                    let offset = child
                        .entries()
                        .first()
                        .map(|e| e.span().offset())
                        .unwrap_or(child.span().offset());
                    credit_expr = Some(parse_expression(&s, offset)?);
                }
                _ => {}
            }
        }
    }

    Ok(PostingLineDef {
        account,
        debit_expr,
        credit_expr,
        span: node.span().into(),
    })
}

fn parse_query(node: &KdlNode, src: &str) -> Result<QueryDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let root_entity = get_prop_string(node, "for", src)?;
    let mut params = vec![];
    let mut selections = vec![];
    let mut formulas = vec![];
    let mut joins = vec![];
    let mut filters = vec![];
    let mut group_by = vec![];

    let query_type = match node.name().value() {
        "query:flat" => QueryType::Flat,
        "query:hierarchy" => QueryType::Hierarchy,
        _ => QueryType::Nested,
    };

    let mut hierarchy = None;

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "params" => {
                    for entry in child.entries() {
                        if let Some(val) = entry.value().as_string() {
                            params.push(val.to_string());
                        }
                    }
                }
                "select" => selections.push(parse_query_selection(child, src)?),
                "formula" => formulas.push(parse_query_formula(child, src)?),
                "join" => joins.push(parse_query_join(child, src)?),
                "filter" => {
                    let s = get_arg_string(child, 0, src)?;
                    let offset = child
                        .entries()
                        .first()
                        .map(|e| e.span().offset())
                        .unwrap_or(child.span().offset());
                    filters.push(parse_expression(&s, offset)?);
                }
                "group_by" => group_by.push(get_arg_string(child, 0, src)?),
                "hierarchy" => hierarchy = Some(parse_hierarchy(child, src)?),
                _ => {}
            }
        }
    }

    Ok(QueryDef {
        name,
        params,
        root_entity,
        query_type,
        selections,
        formulas,
        filters,
        joins,
        group_by,
        hierarchy,
        span: node.span().into(),
    })
}

fn parse_hierarchy(node: &KdlNode, src: &str) -> Result<HierarchyDef, CompileError> {
    let parent_field = get_prop_string(node, "parent", src)?;
    let mut rollup_fields = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            if child.name().value() == "rollup" {
                rollup_fields.push(get_arg_string(child, 0, src)?);
            }
        }
    }

    Ok(HierarchyDef {
        parent_field,
        rollup_fields,
        span: node.span().into(),
    })
}

fn parse_query_selection(node: &KdlNode, src: &str) -> Result<QuerySelectionDef, CompileError> {
    let field = get_arg_string(node, 0, src)?;
    let alias = get_prop_string(node, "as", src).ok();
    Ok(QuerySelectionDef {
        field,
        alias,
        span: node.span().into(),
    })
}

fn parse_query_formula(node: &KdlNode, src: &str) -> Result<QueryFormulaDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let s = get_arg_string(node, 1, src)?;
    let offset = node
        .entries()
        .get(1)
        .map(|e| e.span().offset())
        .unwrap_or(node.span().offset());
    let expression = parse_expression(&s, offset)?;

    Ok(QueryFormulaDef {
        name,
        expression,
        span: node.span().into(),
    })
}

fn parse_query_join(node: &KdlNode, src: &str) -> Result<QueryJoinDef, CompileError> {
    let target_entity = get_arg_string(node, 0, src)?;
    let mut selections = vec![];
    let mut formulas = vec![];
    let mut joins = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "select" => selections.push(parse_query_selection(child, src)?),
                "formula" => formulas.push(parse_query_formula(child, src)?),
                "join" => joins.push(parse_query_join(child, src)?),
                _ => {}
            }
        }
    }

    Ok(QueryJoinDef {
        target_entity,
        selections,
        formulas,
        joins,
        span: node.span().into(),
    })
}

fn parse_enum(node: &KdlNode, src: &str) -> Result<EnumDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut variants = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            variants.push(child.name().value().to_string());
        }
    }

    Ok(EnumDef {
        name,
        variants,
        span: node.span().into(),
    })
}

fn parse_serial_generator(node: &KdlNode, src: &str) -> Result<SerialGeneratorDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut prefix = None;
    let mut date_format = None;
    let mut sequence_digits = 4;

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "prefix" => prefix = Some(get_arg_string(child, 0, src)?),
                "date" => date_format = Some(get_arg_string(child, 0, src)?),
                "sequence" => {
                    if let Ok(digits) = get_prop_int(child, "digits") {
                        sequence_digits = digits as u32;
                    }
                }
                _ => {}
            }
        }
    }

    Ok(SerialGeneratorDef {
        name,
        prefix,
        date_format,
        sequence_digits,
        span: node.span().into(),
    })
}

fn parse_dashboard(node: &KdlNode, src: &str) -> Result<DashboardDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut title = name.clone();
    let mut widgets = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "title" => title = get_arg_string(child, 0, src)?,
                "grid" | "row" => {
                    if let Some(grid_children) = child.children() {
                        for w in grid_children.nodes() {
                            if w.name().value() == "widget" {
                                widgets.push(parse_widget(w, src)?);
                            }
                        }
                    }
                }
                "widget" => widgets.push(parse_widget(child, src)?),
                _ => {}
            }
        }
    }

    Ok(DashboardDef {
        name,
        title,
        widgets,
        span: node.span().into(),
    })
}

fn parse_widget(node: &KdlNode, src: &str) -> Result<WidgetDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let widget_type_str = get_prop_string(node, "type", src)?;
    let widget_type = match widget_type_str.to_lowercase().as_str() {
        "stat" => WidgetType::Stat,
        "chart" => WidgetType::Chart,
        "list" => WidgetType::List,
        "pie" => WidgetType::Pie,
        _ => {
            return Err(CompileError::ParseError {
                src: src.to_string(),
                span: node.span().into(),
                message: format!("Unsupported widget type: {}", widget_type_str),
            });
        }
    };

    let mut label = None;
    let mut value = None;
    let mut icon = None;
    let mut query = None;
    let mut roles = get_prop_string(node, "role", src).ok().map(|r| vec![r]);

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "label" => label = Some(get_arg_string(child, 0, src)?),
                "value" => value = Some(get_arg_string(child, 0, src)?),
                "icon" => icon = Some(get_arg_string(child, 0, src)?),
                "query" => query = Some(get_arg_string(child, 0, src)?),
                "role" => roles = Some(vec![get_arg_string(child, 0, src)?]),
                _ => {}
            }
        }
    }

    Ok(WidgetDef {
        name,
        widget_type,
        label,
        value,
        icon,
        query,
        roles,
        span: node.span().into(),
    })
}

fn parse_page(node: &KdlNode, src: &str) -> Result<PageDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut title = name.clone();
    let mut layout = None;
    let mut content = PageContent::None;

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "title" => title = get_arg_string(child, 0, src)?,
                "layout" => layout = Some(get_arg_string(child, 0, src)?),
                "datatable" => content = PageContent::Datatable(parse_datatable(child, src)?),
                "form" => content = PageContent::Form(parse_form(child, src)?),
                _ => {}
            }
        }
    }

    Ok(PageDef {
        name,
        title,
        layout,
        content,
        span: node.span().into(),
    })
}

fn parse_datatable(node: &KdlNode, src: &str) -> Result<DatatableDef, CompileError> {
    let entity = get_prop_string(node, "for", src).ok();
    let query = get_prop_string(node, "query", src).ok();

    if entity.is_none() && query.is_none() {
        return Err(CompileError::ParseError {
            src: src.to_string(),
            span: node.span().into(),
            message: "Datatable must have either 'for' (entity) or 'query' property".to_string(),
        });
    }

    let mut columns = vec![];
    let mut actions = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "column" => {
                    let field = get_arg_string(child, 0, src)?;
                    let label = get_prop_string(child, "label", src).unwrap_or_else(|_| to_title_case(&field));
                    columns.push(DatatableColumnDef { field, label });
                }
                "action" => {
                    let label = get_arg_string(child, 0, src)?;
                    let icon = get_prop_string(child, "icon", src).ok();
                    let to = get_prop_string(child, "to", src).ok();
                    let method_str = get_prop_string(child, "method", src).ok();
                    let method = match method_str {
                        Some(m) => match m.to_uppercase().as_str() {
                            "GET" => Some(RouteVerb::Get),
                            "POST" => Some(RouteVerb::Post),
                            "PUT" => Some(RouteVerb::Put),
                            "DELETE" => Some(RouteVerb::Delete),
                            _ => {
                                return Err(CompileError::ParseError {
                                    src: src.to_string(),
                                    span: child.span().into(),
                                    message: format!("Unsupported HTTP method: {}", m),
                                });
                            }
                        },
                        None => None,
                    };
                    let variant = get_prop_string(child, "variant", src).ok();

                    actions.push(ActionDef {
                        label,
                        icon,
                        to,
                        method,
                        variant,
                    });
                }
                _ => {}
            }
        }
    }

    Ok(DatatableDef {
        entity,
        query,
        columns,
        actions,
        span: node.span().into(),
    })
}

fn parse_routes(node: &KdlNode, src: &str) -> Result<RoutesDef, CompileError> {
    let mut routes = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            routes.push(parse_route_node(child, src)?);
        }
    }

    Ok(RoutesDef {
        routes,
        span: node.span().into(),
    })
}

fn parse_route_node(node: &KdlNode, src: &str) -> Result<RouteNode, CompileError> {
    match node.name().value() {
        "route" | "route:get" | "route:post" | "route:put" | "route:delete" => {
            let verb = match node.name().value() {
                "route:post" => RouteVerb::Post,
                "route:put" => RouteVerb::Put,
                "route:delete" => RouteVerb::Delete,
                _ => RouteVerb::Get,
            };

            let path = get_arg_string(node, 0, src)?;
            let action = get_prop_string(node, "action", src).or_else(|_| get_prop_string(node, "to", src))?;

            let layout = get_prop_string(node, "layout", src).ok();
            let permission = get_prop_string(node, "permission", src).ok();

            Ok(RouteNode::Route(RouteDef {
                verb,
                path,
                action,
                layout,
                permission,
                span: node.span().into(),
            }))
        }
        "group" => {
            let path = get_arg_string(node, 0, src)?;
            let layout = get_prop_string(node, "layout", src).ok();
            let permission = get_prop_string(node, "permission", src).ok();
            let mut children = vec![];

            if let Some(kids) = node.children() {
                for k in kids.nodes() {
                    children.push(parse_route_node(k, src)?);
                }
            }

            Ok(RouteNode::Group(RouteGroupDef {
                path,
                layout,
                permission,
                children,
                span: node.span().into(),
            }))
        }
        _ => Err(CompileError::ParseError {
            src: src.to_string(),
            span: node.span().into(),
            message: format!("Unexpected node in routes: {}", node.name().value()),
        }),
    }
}

fn parse_menu(node: &KdlNode, src: &str) -> Result<MenuDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut items = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            items.push(parse_menu_item(child, src)?);
        }
    }

    Ok(MenuDef {
        name,
        items,
        span: node.span().into(),
    })
}

fn parse_menu_item(node: &KdlNode, src: &str) -> Result<MenuItemDef, CompileError> {
    let name = node.name().value();
    match name {
        "item" => {
            let label = get_arg_string(node, 0, src)?;
            let to = get_prop_string(node, "to", src)?;
            let icon = get_prop_string(node, "icon", src).ok();

            Ok(MenuItemDef::Item(MenuItem {
                label,
                to,
                icon,
                span: node.span().into(),
            }))
        }
        "group" => {
            let label = get_arg_string(node, 0, src)?;
            let icon = get_prop_string(node, "icon", src).ok();
            let mut children = vec![];

            if let Some(kids) = node.children() {
                for k in kids.nodes() {
                    children.push(parse_menu_item(k, src)?);
                }
            }

            Ok(MenuItemDef::Group(MenuGroup {
                label,
                icon,
                children,
                span: node.span().into(),
            }))
        }
        _ => Err(CompileError::ParseError {
            src: src.to_string(),
            span: node.span().into(),
            message: format!("Unexpected node in menu: {}", name),
        }),
    }
}

fn parse_print(node: &KdlNode, src: &str) -> Result<PrintDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let entity = get_prop_string(node, "for", src)?;
    let mut title = name.clone();

    if let Some(children) = node.children() {
        for child in children.nodes() {
            if child.name().value() == "title" {
                title = get_arg_string(child, 0, src)?;
            }
        }
    }

    Ok(PrintDef {
        name,
        entity,
        title,
        span: node.span().into(),
    })
}

fn parse_form(node: &KdlNode, src: &str) -> Result<FormDef, CompileError> {
    let name = get_arg_string(node, 0, src).unwrap_or_else(|_| "DefaultForm".to_string());
    let entity = get_prop_string(node, "entity", src).or_else(|_| get_prop_string(node, "for", src))?;
    let mut sections = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            if child.name().value() == "section" {
                sections.push(parse_section(child, src)?);
            }
        }
    }

    Ok(FormDef {
        name,
        entity,
        sections,
        span: node.span().into(),
    })
}

fn parse_section(node: &KdlNode, src: &str) -> Result<FormSectionDef, CompileError> {
    let title = get_arg_string(node, 0, src)?;
    let mut fields = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            if child.name().value() == "field" {
                let field_name = get_arg_string(child, 0, src)?;
                fields.push(field_name);
            } else if child.name().value() == "group"
                && let Some(grandkids) = child.children()
            {
                for grandkid in grandkids.nodes() {
                    if grandkid.name().value() == "field" {
                        let field_name = get_arg_string(grandkid, 0, src)?;
                        fields.push(field_name);
                    }
                }
            }
        }
    }

    Ok(FormSectionDef {
        title,
        fields,
        span: node.span().into(),
    })
}

fn parse_permission(node: &KdlNode, src: &str) -> Result<PermissionDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut allows = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            if child.name().value() == "allow" {
                let resource = get_arg_string(child, 0, src)?;
                let actions = get_arg_string(child, 1, src).ok();

                allows.push(AllowDef { resource, actions });
            }
        }
    }

    Ok(PermissionDef {
        name,
        allows,
        span: node.span().into(),
    })
}
