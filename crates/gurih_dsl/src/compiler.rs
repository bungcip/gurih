use crate::ast;
use crate::errors::CompileError;
use crate::parser::parse;
use gurih_ir::{
    ActionSchema, ColumnSchema, DashboardSchema, DatabaseSchema, DatatableColumnSchema, DatatableSchema, EntitySchema,
    FieldSchema, FieldType, FormSchema, FormSection, LayoutSchema, MenuItemSchema, MenuSchema, PageContentSchema,
    PageSchema, PrintSchema, QueryFormula, QueryJoin, QuerySchema, QuerySelection, RelationshipSchema, RouteSchema,
    Schema, SerialGeneratorSchema, StorageSchema, TableSchema, Transition, WidgetSchema, WorkflowSchema,
};
use std::collections::HashMap;

pub fn compile(src: &str, base_path: Option<&std::path::Path>) -> Result<Schema, CompileError> {
    let ast_root = parse(src, base_path)?;

    // Validation Context
    let mut entity_names = HashMap::new();

    let mut ir_entities = HashMap::new();
    let mut ir_tables = HashMap::new();
    let mut ir_modules = HashMap::new();
    let mut ir_workflows = HashMap::new();
    let mut ir_forms = HashMap::new();
    let ir_permissions = HashMap::new();
    let mut ir_layouts = HashMap::new();
    let mut ir_menus = HashMap::new();
    let mut ir_routes = HashMap::new();
    let mut ir_pages = HashMap::new();
    let mut ir_dashboards = HashMap::new();
    let mut ir_serial_generators = HashMap::new();
    let mut ir_prints = HashMap::new();
    let mut ir_storages = HashMap::new();
    let mut ir_queries = HashMap::new();

    // 0. Collect all enums (including from modules)
    let mut enums = HashMap::new();
    for e in &ast_root.enums {
        enums.insert(e.name.clone(), e.variants.clone());
    }
    for m in &ast_root.modules {
        for e in &m.enums {
            enums.insert(e.name.clone(), e.variants.clone());
        }
    }

    // Helper to process entities
    let process_entity =
        |entity_def: &ast::EntityDef, _module_name: Option<&str>| -> Result<EntitySchema, CompileError> {
            let mut fields = vec![];
            let mut field_names = HashMap::new();

            for field_def in &entity_def.fields {
                if field_names.contains_key(&field_def.name) {
                    return Err(CompileError::ValidationError {
                        src: src.to_string(),
                        span: field_def.span,
                        message: format!(
                            "Duplicate field name '{}' in entity '{}'",
                            field_def.name, entity_def.name
                        ),
                    });
                }
                field_names.insert(field_def.name.clone(), field_def.span);

                let field_type = parse_field_type(&field_def.type_name, &enums, &field_def.references)?;
                fields.push(FieldSchema {
                    name: field_def.name.clone(),
                    field_type,
                    required: field_def.required,
                    unique: field_def.unique,
                    default: field_def.default.clone(),
                    references: field_def.references.clone(),
                    serial_generator: field_def.serial_generator.clone(),
                    storage: field_def.storage.clone(),
                    resize: field_def.resize.clone(),
                    filetype: field_def.filetype.clone(),
                });
            }

            let relationships = entity_def
                .relationships
                .iter()
                .map(|r| RelationshipSchema {
                    name: r.name.clone(),
                    target_entity: r.target_entity.clone(),
                    rel_type: match r.rel_type {
                        ast::RelationshipType::BelongsTo => "belongs_to".to_string(),
                        ast::RelationshipType::HasMany => "has_many".to_string(),
                        ast::RelationshipType::HasOne => "has_one".to_string(),
                    },
                })
                .collect();

            let mut options = HashMap::new();
            if entity_def.options.is_submittable {
                options.insert("is_submittable".to_string(), "true".to_string());
            }
            if entity_def.options.track_changes {
                options.insert("track_changes".to_string(), "true".to_string());
            }
            if entity_def.options.is_single {
                options.insert("is_single".to_string(), "true".to_string());
            }

            Ok(EntitySchema {
                name: entity_def.name.clone(),
                fields,
                relationships,
                options,
                seeds: if entity_def.seeds.is_empty() {
                    None
                } else {
                    Some(entity_def.seeds.clone())
                },
            })
        };

    // 0. Process Database
    let database = ast_root.database.map(|d| DatabaseSchema {
        db_type: match d.db_type {
            ast::DatabaseType::Postgres => "postgres".to_string(),
            ast::DatabaseType::Sqlite => "sqlite".to_string(),
            ast::DatabaseType::Unknown(s) => s,
        },
        url: d.url,
    });

    // 1. Process Modules
    for module_def in &ast_root.modules {
        let mut module_entities = vec![];
        for entity_def in &module_def.entities {
            if entity_names.contains_key(&entity_def.name) {
                return Err(CompileError::ValidationError {
                    src: src.to_string(),
                    span: entity_def.span,
                    message: format!("Duplicate entity name: {}", entity_def.name),
                });
            }
            entity_names.insert(entity_def.name.clone(), entity_def.span);
            module_entities.push(entity_def.name.clone());

            let entity_schema = process_entity(entity_def, Some(&module_def.name))?;
            ir_entities.insert(entity_def.name.clone(), entity_schema);
        }

        ir_modules.insert(
            module_def.name.clone(),
            gurih_ir::ModuleSchema {
                name: module_def.name.clone(),
                entities: module_entities,
            },
        );
    }

    // 2. Process Top-Level Entities
    for entity_def in &ast_root.entities {
        if entity_names.contains_key(&entity_def.name) {
            return Err(CompileError::ValidationError {
                src: src.to_string(),
                span: entity_def.span,
                message: format!("Duplicate entity name: {}", entity_def.name),
            });
        }
        entity_names.insert(entity_def.name.clone(), entity_def.span);

        let entity_schema = process_entity(entity_def, None)?;
        ir_entities.insert(entity_def.name.clone(), entity_schema);
    }

    // 3. Process Tables
    for table_def in &ast_root.tables {
        let columns = table_def
            .columns
            .iter()
            .map(|c| ColumnSchema {
                name: c.name.clone(),
                type_name: c.type_name.clone(),
                props: c.props.clone(),
                primary: c.primary,
                unique: c.unique,
            })
            .collect();

        ir_tables.insert(
            table_def.name.clone(),
            TableSchema {
                name: table_def.name.clone(),
                columns,
            },
        );
    }

    // 4. Process Workflows
    for wf_def in &ast_root.workflows {
        ir_workflows.insert(
            wf_def.name.clone(),
            WorkflowSchema {
                name: wf_def.name.clone(),
                entity: wf_def.entity.clone(),
                initial_state: wf_def
                    .states
                    .iter()
                    .find(|s| s.initial)
                    .map(|s| s.name.clone())
                    .unwrap_or_default(),
                states: wf_def.states.iter().map(|s| s.name.clone()).collect(),
                transitions: wf_def
                    .transitions
                    .iter()
                    .map(|t| Transition {
                        name: t.name.clone(),
                        from: t.from.clone(),
                        to: t.to.clone(),
                        required_permission: t.permission.clone(),
                    })
                    .collect(),
            },
        );
    }

    // 5. Process Layouts
    for layout_def in &ast_root.layouts {
        let header_enabled = layout_def.header.as_ref().map(|h| h.enabled).unwrap_or(false);
        let header_props = layout_def.header.as_ref().map(|h| h.props.clone()).unwrap_or_default();

        let props = header_props;

        ir_layouts.insert(
            layout_def.name.clone(),
            LayoutSchema {
                name: layout_def.name.clone(),
                header_enabled,
                sidebar_enabled: layout_def.sidebar.as_ref().map(|s| s.enabled).unwrap_or(false),
                footer: layout_def.footer.clone(),
                props,
            },
        );
    }

    // 6. Process Menus
    fn convert_menu_item(def: &ast::MenuItemDef) -> MenuItemSchema {
        match def {
            ast::MenuItemDef::Item(item) => MenuItemSchema {
                label: item.label.clone(),
                to: Some(item.to.clone()),
                icon: item.icon.clone(),
                children: vec![],
            },
            ast::MenuItemDef::Group(group) => MenuItemSchema {
                label: group.label.clone(),
                to: None,
                icon: group.icon.clone(),
                children: group.children.iter().map(convert_menu_item).collect(),
            },
        }
    }

    for menu_def in &ast_root.menus {
        ir_menus.insert(
            menu_def.name.clone(),
            MenuSchema {
                name: menu_def.name.clone(),
                items: menu_def.items.iter().map(convert_menu_item).collect(),
            },
        );
    }

    // 7. Process Pages
    for page_def in &ast_root.pages {
        let content = match &page_def.content {
            ast::PageContent::Datatable(dt) => PageContentSchema::Datatable(DatatableSchema {
                entity: dt.entity.clone(),
                query: dt.query.clone(),
                columns: dt
                    .columns
                    .iter()
                    .map(|c| DatatableColumnSchema {
                        field: c.field.clone(),
                        label: c.label.clone(),
                    })
                    .collect(),
                actions: dt
                    .actions
                    .iter()
                    .map(|a| ActionSchema {
                        label: a.label.clone(),
                        to: a.to.clone(),
                        method: a.method.clone().map(|m| match m {
                            ast::RouteVerb::Get => "GET".to_string(),
                            ast::RouteVerb::Post => "POST".to_string(),
                            ast::RouteVerb::Put => "PUT".to_string(),
                            ast::RouteVerb::Delete => "DELETE".to_string(),
                        }),
                        icon: a.icon.clone(),
                        variant: a.variant.clone(),
                    })
                    .collect(),
            }),
            ast::PageContent::Form(f) => PageContentSchema::Form(f.name.clone()),
            ast::PageContent::Dashboard => PageContentSchema::Dashboard("".to_string()),
            ast::PageContent::None => PageContentSchema::None,
        };

        if let ast::PageContent::Form(form_def) = &page_def.content {
            let form_name = if form_def.name == "DefaultForm" {
                format!("{}_Form", page_def.name)
            } else {
                form_def.name.clone()
            };

            ir_forms.insert(
                form_name.clone(),
                FormSchema {
                    name: form_name,
                    entity: form_def.entity.clone(),
                    sections: form_def
                        .sections
                        .iter()
                        .map(|s| FormSection {
                            title: s.title.clone(),
                            fields: s.fields.clone(),
                        })
                        .collect(),
                },
            );
        }

        ir_pages.insert(
            page_def.name.clone(),
            PageSchema {
                name: page_def.name.clone(),
                title: page_def.title.clone(),
                content,
            },
        );
    }

    // 8. Process Dashboards
    for db_def in &ast_root.dashboards {
        ir_dashboards.insert(
            db_def.name.clone(),
            DashboardSchema {
                name: db_def.name.clone(),
                title: db_def.title.clone(),
                widgets: db_def
                    .widgets
                    .iter()
                    .map(|w| WidgetSchema {
                        name: w.name.clone(),
                        widget_type: match &w.widget_type {
                            ast::WidgetType::Stat => "stat".to_string(),
                            ast::WidgetType::Chart => "chart".to_string(),
                            ast::WidgetType::List => "list".to_string(),
                            ast::WidgetType::Unknown(s) => s.clone(),
                        },
                        label: w.label.clone(),
                        value: w.value.clone(),
                        icon: w.icon.clone(),
                    })
                    .collect(),
            },
        );
    }

    // 9. Process Serials
    for serial_def in &ast_root.serial_generators {
        ir_serial_generators.insert(
            serial_def.name.clone(),
            SerialGeneratorSchema {
                name: serial_def.name.clone(),
                prefix: serial_def.prefix.clone(),
                digits: serial_def.sequence_digits,
            },
        );
    }

    // 10. Process Action Logic
    let mut ir_actions = HashMap::new();

    let convert_action = |action_def: &ast::ActionLogicDef| -> gurih_ir::ActionLogic {
        let steps = action_def
            .steps
            .iter()
            .map(|s| gurih_ir::ActionStep {
                step_type: match &s.step_type {
                    ast::ActionStepType::EntityDelete => "entity:delete".to_string(),
                    ast::ActionStepType::EntityUpdate => "entity:update".to_string(),
                    ast::ActionStepType::EntityCreate => "entity:create".to_string(),
                    ast::ActionStepType::Custom(s) => s.clone(),
                },
                target: s.target.clone(),
                args: s.args.clone(),
            })
            .collect();

        gurih_ir::ActionLogic {
            name: action_def.name.clone(),
            params: action_def.params.clone(),
            steps,
        }
    };

    for action_def in &ast_root.actions {
        ir_actions.insert(action_def.name.clone(), convert_action(action_def));
    }

    for module_def in &ast_root.modules {
        for action_def in &module_def.actions {
            ir_actions.insert(action_def.name.clone(), convert_action(action_def));
        }
    }

    // 11. Process Routes
    let mut valid_targets: std::collections::HashSet<&str> = ir_pages.keys().map(|s| s.as_str()).collect();
    valid_targets.extend(ir_dashboards.keys().map(|s| s.as_str()));
    valid_targets.extend(ir_actions.keys().map(|s| s.as_str()));

    fn process_route_node(
        node: &ast::RouteNode,
        valid_targets: &std::collections::HashSet<&str>,
        src: &str,
    ) -> Result<RouteSchema, CompileError> {
        match node {
            ast::RouteNode::Route(r) => {
                if !valid_targets.contains(r.action.as_str()) {
                    return Err(CompileError::ValidationError {
                        src: src.to_string(),
                        span: r.span,
                        message: format!("Route target '{}' not found in pages, dashboards or actions", r.action),
                    });
                }
                Ok(RouteSchema {
                    verb: match r.verb {
                        ast::RouteVerb::Get => "GET".to_string(),
                        ast::RouteVerb::Post => "POST".to_string(),
                        ast::RouteVerb::Put => "PUT".to_string(),
                        ast::RouteVerb::Delete => "DELETE".to_string(),
                    },
                    path: r.path.clone(),
                    action: r.action.clone(),
                    layout: r.layout.clone(),
                    permission: r.permission.clone(),
                    children: vec![],
                })
            }
            ast::RouteNode::Group(g) => {
                let mut children = vec![];
                for child in &g.children {
                    children.push(process_route_node(child, valid_targets, src)?);
                }
                Ok(RouteSchema {
                    verb: "ALL".to_string(),
                    path: g.path.clone(),
                    action: "".to_string(),
                    layout: g.layout.clone(),
                    permission: g.permission.clone(),
                    children,
                })
            }
        }
    }

    for routes_def in &ast_root.routes {
        for route_node in &routes_def.routes {
            let schema = process_route_node(route_node, &valid_targets, src)?;
            ir_routes.insert(schema.path.clone(), schema);
        }
    }

    // 12. Process Prints
    for print_def in &ast_root.prints {
        ir_prints.insert(
            print_def.name.clone(),
            PrintSchema {
                name: print_def.name.clone(),
                entity: print_def.entity.clone(),
                title: print_def.title.clone(),
            },
        );
    }

    // 13. Process Storages
    for storage_def in &ast_root.storages {
        ir_storages.insert(
            storage_def.name.clone(),
            StorageSchema {
                name: storage_def.name.clone(),
                driver: match &storage_def.driver {
                    ast::StorageDriver::S3 => "s3".to_string(),
                    ast::StorageDriver::Local => "local".to_string(),
                    ast::StorageDriver::Unknown(s) => s.clone(),
                },
                location: storage_def.location.clone(),
                props: storage_def.props.clone(),
            },
        );
    }

    // 14. Process Queries
    fn convert_expr(e: &crate::expr::Expr) -> gurih_ir::Expression {
        match e {
            crate::expr::Expr::Field(n, _) => gurih_ir::Expression::Field(n.clone()),
            crate::expr::Expr::Literal(n, _) => gurih_ir::Expression::Literal(*n),
            crate::expr::Expr::StringLiteral(s, _) => gurih_ir::Expression::StringLiteral(s.clone()),
            crate::expr::Expr::FunctionCall { name, args, .. } => gurih_ir::Expression::FunctionCall {
                name: name.clone(),
                args: args.iter().map(convert_expr).collect(),
            },
            crate::expr::Expr::BinaryOp { left, op, right, .. } => gurih_ir::Expression::BinaryOp {
                left: Box::new(convert_expr(left)),
                op: match op {
                    crate::expr::BinaryOpType::Add => gurih_ir::BinaryOperator::Add,
                    crate::expr::BinaryOpType::Sub => gurih_ir::BinaryOperator::Sub,
                    crate::expr::BinaryOpType::Mul => gurih_ir::BinaryOperator::Mul,
                    crate::expr::BinaryOpType::Div => gurih_ir::BinaryOperator::Div,
                },
                right: Box::new(convert_expr(right)),
            },
            crate::expr::Expr::Grouping(e, _) => gurih_ir::Expression::Grouping(Box::new(convert_expr(e))),
        }
    }

    fn convert_query_join(def: &ast::QueryJoinDef, src: &str) -> Result<QueryJoin, CompileError> {
        let formulas: Result<Vec<QueryFormula>, CompileError> = def
            .formulas
            .iter()
            .map(|f| {
                let expr = crate::expr::parse_expression(&f.expression, f.span.offset())?;
                Ok(QueryFormula {
                    name: f.name.clone(),
                    expression: convert_expr(&expr),
                })
            })
            .collect();

        let joins: Result<Vec<QueryJoin>, CompileError> =
            def.joins.iter().map(|j| convert_query_join(j, src)).collect();

        Ok(QueryJoin {
            target_entity: def.target_entity.clone(),
            selections: def
                .selections
                .iter()
                .map(|s| QuerySelection {
                    field: s.field.clone(),
                    alias: s.alias.clone(),
                })
                .collect(),
            formulas: formulas?,
            joins: joins?,
        })
    }

    for query_def in &ast_root.queries {
        let formulas: Result<Vec<QueryFormula>, CompileError> = query_def
            .formulas
            .iter()
            .map(|f| {
                let expr = crate::expr::parse_expression(&f.expression, f.span.offset())?;
                Ok(QueryFormula {
                    name: f.name.clone(),
                    expression: convert_expr(&expr),
                })
            })
            .collect();

        let joins: Result<Vec<QueryJoin>, CompileError> =
            query_def.joins.iter().map(|j| convert_query_join(j, src)).collect();

        let filters: Result<Vec<gurih_ir::Expression>, CompileError> = query_def
            .filters
            .iter()
            .map(|f_str| {
                let expr = crate::expr::parse_expression(f_str, 0)?;
                Ok(convert_expr(&expr))
            })
            .collect();

        ir_queries.insert(
            query_def.name.clone(),
            QuerySchema {
                name: query_def.name.clone(),
                root_entity: query_def.root_entity.clone(),
                query_type: match query_def.query_type {
                    ast::QueryType::Nested => gurih_ir::QueryType::Nested,
                    ast::QueryType::Flat => gurih_ir::QueryType::Flat,
                },
                selections: query_def
                    .selections
                    .iter()
                    .map(|s| QuerySelection {
                        field: s.field.clone(),
                        alias: s.alias.clone(),
                    })
                    .collect(),
                formulas: formulas?,
                filters: filters?,
                joins: joins?,
            },
        );
    }

    Ok(Schema {
        name: ast_root.name.unwrap_or("GurihApp".to_string()),
        version: ast_root.version.unwrap_or("1.0.0".to_string()),
        database,
        storages: ir_storages,
        modules: ir_modules,
        entities: ir_entities,
        tables: ir_tables,
        workflows: ir_workflows,
        forms: ir_forms,
        actions: ir_actions,
        permissions: ir_permissions,
        layouts: ir_layouts,
        menus: ir_menus,
        routes: ir_routes,
        pages: ir_pages,
        dashboards: ir_dashboards,
        serial_generators: ir_serial_generators,
        prints: ir_prints,
        queries: ir_queries,
    })
}

fn parse_field_type(
    field_type: &ast::FieldType,
    enums: &HashMap<String, Vec<String>>,
    references: &Option<String>,
) -> Result<FieldType, CompileError> {
    match field_type {
        ast::FieldType::String
        | ast::FieldType::Code
        | ast::FieldType::Serial
        | ast::FieldType::Money
        | ast::FieldType::Email
        | ast::FieldType::Phone
        | ast::FieldType::Name
        | ast::FieldType::Description => Ok(FieldType::String),
        ast::FieldType::Text => Ok(FieldType::Text),
        ast::FieldType::Integer => Ok(FieldType::Integer),
        ast::FieldType::Float => Ok(FieldType::Float),
        ast::FieldType::Boolean => Ok(FieldType::Boolean),
        ast::FieldType::Date => Ok(FieldType::Date),
        ast::FieldType::DateTime => Ok(FieldType::DateTime),
        ast::FieldType::Password => Ok(FieldType::Password),
        ast::FieldType::Relation => Ok(FieldType::Relation),
        ast::FieldType::Photo => Ok(FieldType::Photo),
        ast::FieldType::File => Ok(FieldType::File),
        ast::FieldType::Enum => {
            // For explicit Enum type, references should be set to the enum name.
            let variants = if let Some(ref_name) = references {
                enums.get(ref_name).cloned().unwrap_or_default()
            } else {
                vec![]
            };
            Ok(FieldType::Enum(variants))
        }
        ast::FieldType::Custom(s) => {
            // Check if it matches an enum name
            if let Some(variants) = enums.get(s) {
                Ok(FieldType::Enum(variants.clone()))
            } else {
                // If unknown, default to string
                Ok(FieldType::String)
            }
        }
    }
}
