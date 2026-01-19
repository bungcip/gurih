use crate::ast;
use crate::errors::CompileError;
use crate::parser::parse;
use gurih_ir::{
    ActionSchema, ColumnSchema, DashboardSchema, DatabaseSchema, DatatableColumnSchema,
    DatatableSchema, EntitySchema, FieldSchema, FieldType, FormSchema, FormSection, LayoutSchema,
    MenuItemSchema, MenuSchema, PageContentSchema, PageSchema, PrintSchema, RelationshipSchema,
    RouteSchema, Schema, SerialSchema, TableSchema, Transition, WidgetSchema, WorkflowSchema,
};
use std::collections::HashMap;

pub fn compile(src: &str) -> Result<Schema, CompileError> {
    let ast_root = parse(src)?;

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
    let mut ir_serials = HashMap::new();
    let mut ir_prints = HashMap::new();

    // Helper to process entities
    let process_entity = |entity_def: &ast::EntityDef,
                          _module_name: Option<&str>|
     -> Result<EntitySchema, CompileError> {
        let mut fields = vec![];
        for field_def in &entity_def.fields {
            let field_type = parse_field_type(&field_def.type_name, src, &field_def.span)?;
            fields.push(FieldSchema {
                name: field_def.name.clone(),
                field_type,
                required: field_def.required,
                unique: field_def.unique,
                default: field_def.default.clone(),
                references: field_def.references.clone(),
                serial: field_def.serial.clone(),
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
        })
    };

    // 0. Process Database
    let database = ast_root.database.map(|d| DatabaseSchema {
        db_type: d.db_type,
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
        // ... (validation logic similar to before)
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
        let header_enabled = layout_def
            .header
            .as_ref()
            .map(|h| h.enabled)
            .unwrap_or(false);
        let header_props = layout_def
            .header
            .as_ref()
            .map(|h| h.props.clone())
            .unwrap_or_default();

        let props = header_props; // Merge props? Simple flattening for now

        ir_layouts.insert(
            layout_def.name.clone(),
            LayoutSchema {
                name: layout_def.name.clone(),
                header_enabled,
                sidebar_enabled: layout_def
                    .sidebar
                    .as_ref()
                    .map(|s| s.enabled)
                    .unwrap_or(false),
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
                        icon: a.icon.clone(),
                    })
                    .collect(),
            }),
            ast::PageContent::Form(f) => PageContentSchema::Form(f.name.clone()), // Logic ref: Store form definition separately?
            ast::PageContent::Dashboard => PageContentSchema::Dashboard("".to_string()), // Placeholder
            ast::PageContent::None => PageContentSchema::None,
        };

        // If page has an embedded form definition, add it to ir_forms
        if let ast::PageContent::Form(form_def) = &page_def.content {
            ir_forms.insert(
                form_def.name.clone(),
                FormSchema {
                    name: form_def.name.clone(),
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
                        widget_type: w.widget_type.clone(),
                        label: w.label.clone(),
                        value: w.value.clone(),
                    })
                    .collect(),
            },
        );
    }

    // 9. Process Serials
    for serial_def in &ast_root.serials {
        ir_serials.insert(
            serial_def.name.clone(),
            SerialSchema {
                name: serial_def.name.clone(),
                prefix: serial_def.prefix.clone(),
                digits: serial_def.sequence_digits,
            },
        );
    }

    // 10. Process Routes
    // Flatten routes for schema
    let mut valid_pages: std::collections::HashSet<&str> =
        ir_pages.keys().map(|s| s.as_str()).collect();
    valid_pages.extend(ir_dashboards.keys().map(|s| s.as_str()));

    fn process_route_node(
        node: &ast::RouteNode,
        valid_pages: &std::collections::HashSet<&str>,
        src: &str,
    ) -> Result<RouteSchema, CompileError> {
        match node {
            ast::RouteNode::Route(r) => {
                if !valid_pages.contains(r.to.as_str()) {
                    return Err(CompileError::ValidationError {
                        src: src.to_string(),
                        span: r.span,
                        message: format!("Route target '{}' not found in pages", r.to),
                    });
                }
                Ok(RouteSchema {
                    path: r.path.clone(),
                    to: r.to.clone(),
                    layout: r.layout.clone(),
                    permission: r.permission.clone(),
                    children: vec![],
                })
            }
            ast::RouteNode::Group(g) => {
                let mut children = vec![];
                for child in &g.children {
                    children.push(process_route_node(child, valid_pages, src)?);
                }
                Ok(RouteSchema {
                    path: g.path.clone(),
                    to: "".to_string(), // Group doesn't point to page usually
                    layout: g.layout.clone(),
                    permission: g.permission.clone(),
                    children,
                })
            }
        }
    }

    for routes_def in &ast_root.routes {
        for route_node in &routes_def.routes {
            let schema = process_route_node(route_node, &valid_pages, src)?;
            // Use path as key? Or accumulate in a list.
            // Schema defines routes: HashMap<String, RouteSchema>.
            ir_routes.insert(schema.path.clone(), schema);
        }
    }

    // 11. Process Prints
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

    Ok(Schema {
        name: ast_root.name.unwrap_or("GurihApp".to_string()),
        version: ast_root.version.unwrap_or("1.0.0".to_string()),
        database,
        modules: ir_modules,
        entities: ir_entities,
        tables: ir_tables,
        workflows: ir_workflows,
        forms: ir_forms,
        permissions: ir_permissions,
        layouts: ir_layouts,
        menus: ir_menus,
        routes: ir_routes,
        pages: ir_pages,
        dashboards: ir_dashboards,
        serials: ir_serials,
        prints: ir_prints,
    })
}

fn parse_field_type(
    type_name: &str,
    _src: &str,
    _span: &crate::diagnostics::SourceSpan,
) -> Result<FieldType, CompileError> {
    match type_name {
        "String" | "Code" | "Money" => Ok(FieldType::String), // Money/Code as string for now
        "Text" => Ok(FieldType::Text),
        "Integer" => Ok(FieldType::Integer),
        "Float" => Ok(FieldType::Float),
        "Boolean" => Ok(FieldType::Boolean),
        "Date" => Ok(FieldType::Date),
        "DateTime" => Ok(FieldType::DateTime),
        "Relation" => Ok(FieldType::Relation),
        "Enum" => Ok(FieldType::Enum(vec![])), // Placeholder, real enum lookup might be needed
        _ => {
            // If unknown, default to string or error.
            // Since we have semantic types like "Time", "Decimal" not explicitly matched.
            Ok(FieldType::String)
        }
    }
}
