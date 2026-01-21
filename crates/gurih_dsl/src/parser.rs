use crate::ast::*;
use crate::diagnostics::SourceSpan;
use crate::errors::CompileError;
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
            "query" | "query:nested" | "query:flat" => ast.queries.push(parse_query(node, src)?),
            _ => {
                // Ignore unknown nodes or warn? Strict for now.
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
                    // step "entity:delete" target="Position" ...
                    let step_type_str = get_arg_string(child, 0, src)?;
                    let span = child.span();
                    let step_type = parse_step_type(&step_type_str, span.into(), src)?;
                    let target = get_prop_string(child, "target", src)?;
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
                    // e.g. step:entity:delete "Position" id=param("id")
                    // target is arg 0
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

fn parse_step_type(s: &str, span: SourceSpan, src: &str) -> Result<ActionStepType, CompileError> {
    match s {
        "entity:delete" => Ok(ActionStepType::EntityDelete),
        "entity:update" => Ok(ActionStepType::EntityUpdate),
        "entity:create" => Ok(ActionStepType::EntityCreate),
        _ => Err(CompileError::ParseError {
            src: src.to_string(),
            span: span.into(),
            message: format!("Unknown action step type: {}", s),
        }),
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
                        (arg0.to_lowercase(), arg0.clone())
                    };

                    relationships.push(RelationshipDef {
                        rel_type,
                        name,
                        target_entity: target,
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

                    let required = get_prop_bool(child, "required").unwrap_or(false);
                    let unique = get_prop_bool(child, "unique").unwrap_or(false);
                    let default = get_prop_string(child, "default", src).ok();
                    let serial_generator = get_prop_string(child, "serial_generator", src).ok();

                    let references = if child_name == "enum" {
                        Some(get_arg_string(child, 1, src)?)
                    } else {
                        get_prop_string(child, "references", src).ok()
                    };

                    let storage = get_prop_string(child, "storage", src).ok();
                    let resize = get_prop_string(child, "resize", src).ok();
                    let filetype = get_prop_string(child, "filetype", src).ok();

                    fields.push(FieldDef {
                        name: field_name,
                        type_name,
                        serial_generator,
                        required,
                        unique,
                        default,
                        references,
                        storage,
                        resize,
                        filetype,
                        span: child.span().into(),
                    });
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

    let references = if node.name().value() == "enum" || node.name().value() == "field:enum" {
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
        "enum" => FieldType::Enum,
        "int" | "integer" => FieldType::Integer,
        "float" | "decimal" => FieldType::Float,
        "date" => FieldType::Date,
        "datetime" | "timestamp" => FieldType::Timestamp,
        "string" => FieldType::String,
        "text" => FieldType::Text,
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

    Ok(StateDef {
        name,
        initial,
        span: node.span().into(),
    })
}

fn parse_transition(node: &KdlNode, src: &str) -> Result<TransitionDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let from = get_prop_string(node, "from", src)?;
    let to = get_prop_string(node, "to", src)?;
    let permission = get_prop_string(node, "permission", src).ok();

    Ok(TransitionDef {
        name,
        from,
        to,
        permission,
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

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "label" => label = Some(get_arg_string(child, 0, src)?),
                "value" => value = Some(get_arg_string(child, 0, src)?),
                "icon" => icon = Some(get_arg_string(child, 0, src)?),
                "query" => query = Some(get_arg_string(child, 0, src)?),
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

fn parse_query(node: &KdlNode, src: &str) -> Result<QueryDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let root_entity = get_prop_string(node, "for", src)?;
    let mut selections = vec![];
    let mut formulas = vec![];
    let mut joins = vec![];
    let mut filters = vec![];

    let query_type = match node.name().value() {
        "query:flat" => QueryType::Flat,
        _ => QueryType::Nested,
    };

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "select" => selections.push(parse_query_selection(child, src)?),
                "formula" => formulas.push(parse_query_formula(child, src)?),
                "join" => joins.push(parse_query_join(child, src)?),
                "filter" => filters.push(get_arg_string(child, 0, src)?),
                _ => {}
            }
        }
    }

    Ok(QueryDef {
        name,
        root_entity,
        query_type,
        selections,
        formulas,
        filters,
        joins,
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
    let expression = get_arg_string(node, 1, src)?;
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

// Helpers

fn get_arg_string(node: &KdlNode, index: usize, src: &str) -> Result<String, CompileError> {
    node.entry(index)
        .and_then(|val| val.value().as_string().map(|s| s.to_string()))
        .ok_or_else(|| CompileError::ParseError {
            src: src.to_string(),
            span: node.span().into(),
            message: format!(
                "Missing or invalid argument at index {} for node '{}'",
                index,
                node.name().value()
            ),
        })
}

fn get_prop_string(node: &KdlNode, key: &str, src: &str) -> Result<String, CompileError> {
    node.get(key)
        .and_then(|val| val.as_string().map(|s| s.to_string()))
        .ok_or_else(|| CompileError::ParseError {
            src: src.to_string(),
            span: node.span().into(),
            message: format!("Missing property '{}'", key),
        })
}

fn get_prop_bool(node: &KdlNode, key: &str) -> Option<bool> {
    node.get(key).and_then(|val| {
        if let Some(b) = val.as_bool() {
            Some(b)
        } else if let Some(s) = val.as_string() {
            match s {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn get_prop_int(node: &KdlNode, key: &str) -> Result<i64, CompileError> {
    node.get(key)
        .and_then(|val| val.as_integer().map(|i| i as i64))
        .ok_or_else(|| CompileError::ParseError {
            src: "".to_string(), // context missing here, ideally pass src
            span: node.span().into(),
            message: format!("Missing or invalid int property '{}'", key),
        })
}

fn get_arg_bool(node: &KdlNode, index: usize) -> Result<bool, CompileError> {
    node.entry(index)
        .and_then(|val| {
            if let Some(b) = val.value().as_bool() {
                Some(b)
            } else if let Some(s) = val.value().as_string() {
                match s {
                    "true" => Some(true),
                    "false" => Some(false),
                    _ => None,
                }
            } else {
                None
            }
        })
        .ok_or_else(|| CompileError::ParseError {
            src: "".to_string(),
            span: node.span().into(),
            message: format!("Missing or invalid bool argument at index {}", index),
        })
}

fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
