use crate::ast;
use crate::errors::CompileError;
use crate::parser::parse;
use crate::validator::Validator;
use gurih_ir::Symbol;
use gurih_ir::{
    ActionSchema, ColumnSchema, DashboardSchema, DatabaseSchema, DatatableColumnSchema, DatatableSchema, EntitySchema,
    FieldSchema, FieldType, FormSchema, FormSection, LayoutSchema, MenuItemSchema, MenuSchema, PageContentSchema,
    PageSchema, PermissionSchema, PrintSchema, QueryFormula, QueryJoin, QuerySchema, QuerySelection,
    RelationshipSchema, RouteSchema, Schema, SerialGeneratorSchema, StateSchema, StorageSchema, TableSchema,
    Transition, TransitionEffect, TransitionPrecondition, WidgetSchema, WorkflowSchema,
};
use std::collections::HashMap;

pub fn compile(src: &str, base_path: Option<&std::path::Path>) -> Result<Schema, CompileError> {
    let ast_root = parse(src, base_path)?;

    // Run Validation
    Validator::new(src).validate(&ast_root)?;

    let mut ir_entities: HashMap<Symbol, EntitySchema> = HashMap::new();
    let mut ir_tables: HashMap<Symbol, TableSchema> = HashMap::new();
    let mut ir_modules: HashMap<Symbol, gurih_ir::ModuleSchema> = HashMap::new();
    let mut ir_workflows: HashMap<Symbol, WorkflowSchema> = HashMap::new();
    let mut ir_forms: HashMap<Symbol, FormSchema> = HashMap::new();
    let ir_permissions: HashMap<Symbol, PermissionSchema> = HashMap::new();
    let mut ir_layouts: HashMap<Symbol, LayoutSchema> = HashMap::new();
    let mut ir_menus: HashMap<Symbol, MenuSchema> = HashMap::new();
    let mut ir_routes: HashMap<String, RouteSchema> = HashMap::new();
    let mut ir_pages: HashMap<Symbol, PageSchema> = HashMap::new();
    let mut ir_dashboards: HashMap<Symbol, DashboardSchema> = HashMap::new();
    let mut ir_serial_generators: HashMap<Symbol, SerialGeneratorSchema> = HashMap::new();
    let mut ir_prints: HashMap<Symbol, PrintSchema> = HashMap::new();
    let mut ir_storages: HashMap<Symbol, StorageSchema> = HashMap::new();
    let mut ir_queries: HashMap<Symbol, QuerySchema> = HashMap::new();

    // 0. Collect all enums (including from modules)
    let mut enums: HashMap<String, Vec<Symbol>> = HashMap::new();
    for e in &ast_root.enums {
        enums.insert(
            e.name.clone(),
            e.variants.iter().map(|v| Symbol::from(v.as_str())).collect(),
        );
    }
    for m in &ast_root.modules {
        for e in &m.enums {
            enums.insert(
                e.name.clone(),
                e.variants.iter().map(|v| Symbol::from(v.as_str())).collect(),
            );
        }
    }

    // Helper to process entities
    let process_entity =
        |entity_def: &ast::EntityDef, _module_name: Option<&str>| -> Result<EntitySchema, CompileError> {
            let mut fields = vec![];

            for field_def in &entity_def.fields {
                let field_type = parse_field_type(&field_def.type_name, &enums, &field_def.references)?;

                fields.push(FieldSchema {
                    name: field_def.name.as_str().into(),
                    field_type,
                    required: field_def.required,
                    unique: field_def.unique,
                    default: field_def.default.clone(),
                    references: field_def.references.as_ref().map(|s| s.as_str().into()),
                    serial_generator: field_def.serial_generator.as_ref().map(|s| s.as_str().into()),
                    storage: field_def.storage.as_ref().map(|s| s.as_str().into()),
                    resize: field_def.resize.clone(),
                    filetype: field_def.filetype.clone(),
                });
            }

            let relationships = entity_def
                .relationships
                .iter()
                .map(|r| RelationshipSchema {
                    name: r.name.as_str().into(),
                    target_entity: r.target_entity.as_str().into(),
                    rel_type: r.rel_type.clone(),
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
                name: entity_def.name.as_str().into(),
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
        db_type: d.db_type,
        url: d.url,
    });

    // 1. Process Modules
    for module_def in &ast_root.modules {
        let mut module_entities = vec![];
        for entity_def in &module_def.entities {
            module_entities.push(entity_def.name.clone());
            let entity_schema = process_entity(entity_def, Some(&module_def.name))?;
            ir_entities.insert(entity_def.name.as_str().into(), entity_schema);
        }

        ir_modules.insert(
            module_def.name.as_str().into(),
            gurih_ir::ModuleSchema {
                name: module_def.name.as_str().into(),
                entities: module_entities.iter().map(|s| Symbol::from(s.as_str())).collect(),
            },
        );
    }

    // 4.1 Process Employee Statuses (synthesize workflow)
    if !ast_root.employee_statuses.is_empty() {
        let mut grouped_statuses: HashMap<(String, String), Vec<&ast::EmployeeStatusDef>> = HashMap::new();

        for status_def in &ast_root.employee_statuses {
            let entity = status_def.entity.clone().unwrap_or_else(|| "Employee".to_string());
            let field = status_def.field.clone().unwrap_or_else(|| "status".to_string());
            grouped_statuses.entry((entity, field)).or_default().push(status_def);
        }

        for ((entity_name, field_name), statuses) in grouped_statuses {
            let workflow_name = format!("{}StatusWorkflow", entity_name);

            let mut states: Vec<StateSchema> = vec![];
            let mut transitions: Vec<Transition> = vec![];

            // Track seen states to avoid duplicates
            let mut seen_states: std::collections::HashSet<String> = std::collections::HashSet::new();

            for status_def in statuses {
                if seen_states.insert(status_def.name.clone()) {
                    states.push(StateSchema {
                        name: Symbol::from(status_def.name.as_str()),
                        immutable: false,
                    });
                }

                for trans_def in &status_def.transitions {
                    if seen_states.insert(trans_def.to.clone()) {
                        states.push(StateSchema {
                            name: Symbol::from(trans_def.to.as_str()),
                            immutable: false,
                        });
                    }

                    transitions.push(Transition {
                        name: format!("{}_to_{}", status_def.name, trans_def.to).into(),
                        from: Symbol::from(status_def.name.as_str()),
                        to: Symbol::from(trans_def.to.as_str()),
                        required_permission: trans_def.permission.as_ref().map(|p| Symbol::from(p.as_str())),
                        preconditions: trans_def
                            .preconditions
                            .iter()
                            .map(|p| match p {
                                ast::TransitionPreconditionDef::Document { name, .. } => {
                                    TransitionPrecondition::Document(Symbol::from(name.as_str()))
                                }
                                ast::TransitionPreconditionDef::MinYearsOfService { years, from_field, .. } => {
                                    TransitionPrecondition::MinYearsOfService {
                                        years: *years,
                                        from_field: from_field.as_ref().map(|s| Symbol::from(s.as_str())),
                                    }
                                }
                                ast::TransitionPreconditionDef::MinAge {
                                    age, birth_date_field, ..
                                } => TransitionPrecondition::MinAge {
                                    age: *age,
                                    birth_date_field: birth_date_field.as_ref().map(|s| Symbol::from(s.as_str())),
                                },
                                ast::TransitionPreconditionDef::ValidEffectiveDate { field, .. } => {
                                    TransitionPrecondition::ValidEffectiveDate(Symbol::from(field.as_str()))
                                }
                                ast::TransitionPreconditionDef::BalancedTransaction { .. } => {
                                    TransitionPrecondition::BalancedTransaction
                                }
                                ast::TransitionPreconditionDef::PeriodOpen { entity, .. } => {
                                    TransitionPrecondition::PeriodOpen {
                                        entity: entity.as_ref().map(|s| Symbol::from(s.as_str())),
                                    }
                                }
                            })
                            .collect(),
                        effects: trans_def
                            .effects
                            .iter()
                            .map(|e| match e {
                                ast::TransitionEffectDef::SuspendPayroll { active, .. } => {
                                    TransitionEffect::SuspendPayroll(*active)
                                }
                                ast::TransitionEffectDef::Notify { target, .. } => {
                                    TransitionEffect::Notify(Symbol::from(target.as_str()))
                                }
                                ast::TransitionEffectDef::UpdateRankEligibility { active, .. } => {
                                    TransitionEffect::UpdateRankEligibility(*active)
                                }
                                ast::TransitionEffectDef::UpdateField { field, value, .. } => {
                                    TransitionEffect::UpdateField {
                                        field: Symbol::from(field.as_str()),
                                        value: value.clone(),
                                    }
                                }
                            })
                            .collect(),
                    });
                }
            }

            ir_workflows.insert(
                Symbol::from(workflow_name.as_str()),
                WorkflowSchema {
                    name: Symbol::from(workflow_name.as_str()),
                    entity: Symbol::from(entity_name.as_str()),
                    field: Symbol::from(field_name.as_str()),
                    initial_state: states.first().map(|s| s.name).unwrap_or_else(|| Symbol::from("")),
                    states,
                    transitions,
                },
            );
        }
    }

    // 2. Process Top-Level Entities
    for entity_def in &ast_root.entities {
        let entity_schema = process_entity(entity_def, None)?;
        ir_entities.insert(entity_def.name.as_str().into(), entity_schema);
    }

    // 2.1 Process Accounts (inject into Account entity seeds)
    if !ast_root.accounts.is_empty() {
        let account_symbol = Symbol::from("Account");
        if let Some(entity) = ir_entities.get_mut(&account_symbol) {
            let mut seeds = entity.seeds.take().unwrap_or_default();
            for acc in &ast_root.accounts {
                let mut row = acc.fields.clone();
                row.insert("name".to_string(), acc.name.clone());
                seeds.push(row);
            }
            entity.seeds = Some(seeds);
        } else {
            // Warn or Error? For now error to enforce definition.
            return Err(CompileError::ParseError {
                src: src.to_string(),
                span: ast_root.accounts[0].span,
                message: "Found 'account' definitions but no 'Account' entity defined.".to_string(),
            });
        }
    }

    // 3. Process Tables
    for table_def in &ast_root.tables {
        let columns = table_def
            .columns
            .iter()
            .map(|c| ColumnSchema {
                name: c.name.as_str().into(),
                type_name: c.type_name.clone(),
                props: c.props.clone(),
                primary: c.primary,
                unique: c.unique,
            })
            .collect();

        ir_tables.insert(
            table_def.name.as_str().into(),
            TableSchema {
                name: table_def.name.as_str().into(),
                columns,
            },
        );
    }

    // 4. Process Workflows
    for wf_def in &ast_root.workflows {
        ir_workflows.insert(
            wf_def.name.as_str().into(),
            WorkflowSchema {
                name: wf_def.name.as_str().into(),
                entity: wf_def.entity.as_str().into(),
                field: wf_def.field.as_str().into(),
                initial_state: wf_def
                    .states
                    .iter()
                    .find(|s| s.initial)
                    .map(|s| s.name.as_str().into())
                    .unwrap_or_else(|| Symbol::from("")),
                states: wf_def
                    .states
                    .iter()
                    .map(|s| StateSchema {
                        name: Symbol::from(s.name.as_str()),
                        immutable: s.immutable,
                    })
                    .collect(),
                transitions: wf_def
                    .transitions
                    .iter()
                    .map(|t| Transition {
                        name: t.name.as_str().into(),
                        from: t.from.as_str().into(),
                        to: t.to.as_str().into(),
                        required_permission: t.permission.as_ref().map(|p| Symbol::from(p.as_str())),
                        preconditions: t
                            .preconditions
                            .iter()
                            .map(|p| match p {
                                ast::TransitionPreconditionDef::Document { name, .. } => {
                                    TransitionPrecondition::Document(Symbol::from(name.as_str()))
                                }
                                ast::TransitionPreconditionDef::MinYearsOfService { years, from_field, .. } => {
                                    TransitionPrecondition::MinYearsOfService {
                                        years: *years,
                                        from_field: from_field.as_ref().map(|s| Symbol::from(s.as_str())),
                                    }
                                }
                                ast::TransitionPreconditionDef::MinAge {
                                    age, birth_date_field, ..
                                } => TransitionPrecondition::MinAge {
                                    age: *age,
                                    birth_date_field: birth_date_field.as_ref().map(|s| Symbol::from(s.as_str())),
                                },
                                ast::TransitionPreconditionDef::ValidEffectiveDate { field, .. } => {
                                    TransitionPrecondition::ValidEffectiveDate(Symbol::from(field.as_str()))
                                }
                                ast::TransitionPreconditionDef::BalancedTransaction { .. } => {
                                    TransitionPrecondition::BalancedTransaction
                                }
                                ast::TransitionPreconditionDef::PeriodOpen { entity, .. } => {
                                    TransitionPrecondition::PeriodOpen {
                                        entity: entity.as_ref().map(|s| Symbol::from(s.as_str())),
                                    }
                                }
                            })
                            .collect(),
                        effects: t
                            .effects
                            .iter()
                            .map(|e| match e {
                                ast::TransitionEffectDef::SuspendPayroll { active, .. } => {
                                    TransitionEffect::SuspendPayroll(*active)
                                }
                                ast::TransitionEffectDef::Notify { target, .. } => {
                                    TransitionEffect::Notify(Symbol::from(target.as_str()))
                                }
                                ast::TransitionEffectDef::UpdateRankEligibility { active, .. } => {
                                    TransitionEffect::UpdateRankEligibility(*active)
                                }
                                ast::TransitionEffectDef::UpdateField { field, value, .. } => {
                                    TransitionEffect::UpdateField {
                                        field: Symbol::from(field.as_str()),
                                        value: value.clone(),
                                    }
                                }
                            })
                            .collect(),
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
            layout_def.name.as_str().into(),
            LayoutSchema {
                name: layout_def.name.as_str().into(),
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
            menu_def.name.as_str().into(),
            MenuSchema {
                name: menu_def.name.as_str().into(),
                items: menu_def.items.iter().map(convert_menu_item).collect(),
            },
        );
    }

    // 7. Process Pages
    for page_def in &ast_root.pages {
        let content = match &page_def.content {
            ast::PageContent::Datatable(dt) => PageContentSchema::Datatable(DatatableSchema {
                entity: dt.entity.as_ref().map(|s| s.as_str().into()),
                query: dt.query.as_ref().map(|s| s.as_str().into()),
                columns: dt
                    .columns
                    .iter()
                    .map(|c| DatatableColumnSchema {
                        field: c.field.as_str().into(),
                        label: c.label.clone(),
                    })
                    .collect(),
                actions: dt
                    .actions
                    .iter()
                    .map(|a| ActionSchema {
                        label: a.label.clone(),
                        to: a.to.as_ref().map(|s| s.as_str().into()),
                        method: a.method.clone(),
                        icon: a.icon.clone(),
                        variant: a.variant.clone(),
                    })
                    .collect(),
            }),
            ast::PageContent::Form(f) => PageContentSchema::Form(f.name.as_str().into()),
            ast::PageContent::Dashboard => PageContentSchema::Dashboard(Symbol::from("")),
            ast::PageContent::None => PageContentSchema::None,
        };

        if let ast::PageContent::Form(form_def) = &page_def.content {
            let form_name = if form_def.name == "DefaultForm" {
                format!("{}_Form", page_def.name)
            } else {
                form_def.name.clone()
            };

            ir_forms.insert(
                form_name.as_str().into(),
                FormSchema {
                    name: form_name.as_str().into(),
                    entity: form_def.entity.as_str().into(),
                    sections: form_def
                        .sections
                        .iter()
                        .map(|s| FormSection {
                            title: s.title.clone(),
                            fields: s.fields.iter().map(|f| Symbol::from(f.as_str())).collect(),
                        })
                        .collect(),
                },
            );
        }

        ir_pages.insert(
            page_def.name.as_str().into(),
            PageSchema {
                name: page_def.name.as_str().into(),
                title: page_def.title.clone(),
                content,
            },
        );
    }

    // 8. Process Dashboards
    for db_def in &ast_root.dashboards {
        ir_dashboards.insert(
            db_def.name.as_str().into(),
            DashboardSchema {
                name: db_def.name.as_str().into(),
                title: db_def.title.clone(),
                widgets: db_def
                    .widgets
                    .iter()
                    .map(|w| WidgetSchema {
                        name: w.name.as_str().into(),
                        widget_type: w.widget_type.clone(),
                        label: w.label.clone(),
                        value: w.value.clone(),
                        icon: w.icon.clone(),
                        roles: w.roles.clone(),
                    })
                    .collect(),
            },
        );
    }

    // 9. Process Serials
    for serial_def in &ast_root.serial_generators {
        ir_serial_generators.insert(
            serial_def.name.as_str().into(),
            SerialGeneratorSchema {
                name: serial_def.name.as_str().into(),
                prefix: serial_def.prefix.clone(),
                digits: serial_def.sequence_digits,
            },
        );
    }

    // 10. Process Action Logic
    let mut ir_actions: HashMap<Symbol, gurih_ir::ActionLogic> = HashMap::new();

    let convert_action = |action_def: &ast::ActionLogicDef| -> gurih_ir::ActionLogic {
        let steps = action_def
            .steps
            .iter()
            .map(|s| gurih_ir::ActionStep {
                step_type: s.step_type.clone(),
                target: s.target.as_str().into(),
                args: s.args.clone(),
            })
            .collect();

        gurih_ir::ActionLogic {
            name: action_def.name.as_str().into(),
            params: action_def.params.iter().map(|p| Symbol::from(p.as_str())).collect(),
            steps,
        }
    };

    for action_def in &ast_root.actions {
        ir_actions.insert(action_def.name.as_str().into(), convert_action(action_def));
    }

    for module_def in &ast_root.modules {
        for action_def in &module_def.actions {
            ir_actions.insert(action_def.name.as_str().into(), convert_action(action_def));
        }
    }

    // 11. Process Routes
    fn process_route_node(node: &ast::RouteNode, _src: &str) -> Result<RouteSchema, CompileError> {
        match node {
            ast::RouteNode::Route(r) => Ok(RouteSchema {
                verb: r.verb.clone(),
                path: r.path.clone(),
                action: r.action.as_str().into(),
                layout: r.layout.as_ref().map(|l| l.as_str().into()),
                permission: r.permission.as_ref().map(|p| p.as_str().into()),
                children: vec![],
            }),
            ast::RouteNode::Group(g) => {
                let mut children = vec![];
                for child in &g.children {
                    children.push(process_route_node(child, _src)?);
                }
                Ok(RouteSchema {
                    verb: gurih_ir::RouteVerb::All,
                    path: g.path.clone(),
                    action: Symbol::from(""),
                    layout: g.layout.as_ref().map(|l| l.as_str().into()),
                    permission: g.permission.as_ref().map(|p| p.as_str().into()),
                    children,
                })
            }
        }
    }

    for routes_def in &ast_root.routes {
        for route_node in &routes_def.routes {
            let schema = process_route_node(route_node, src)?;
            ir_routes.insert(schema.path.clone(), schema);
        }
    }

    // 12. Process Prints
    for print_def in &ast_root.prints {
        ir_prints.insert(
            print_def.name.as_str().into(),
            PrintSchema {
                name: print_def.name.as_str().into(),
                entity: print_def.entity.as_str().into(),
                title: print_def.title.clone(),
            },
        );
    }

    // 13. Process Storages
    for storage_def in &ast_root.storages {
        ir_storages.insert(
            storage_def.name.as_str().into(),
            StorageSchema {
                name: storage_def.name.as_str().into(),
                driver: storage_def.driver.clone(),
                location: storage_def.location.clone(),
                props: storage_def.props.clone(),
            },
        );
    }

    // 14. Process Queries
    fn convert_expr(e: &crate::expr::Expr) -> gurih_ir::Expression {
        match e {
            crate::expr::Expr::Field(n, _) => gurih_ir::Expression::Field(n.as_str().into()),
            crate::expr::Expr::Literal(n, _) => gurih_ir::Expression::Literal(*n),
            crate::expr::Expr::StringLiteral(s, _) => gurih_ir::Expression::StringLiteral(s.clone()),
            crate::expr::Expr::FunctionCall { name, args, .. } => gurih_ir::Expression::FunctionCall {
                name: name.as_str().into(),
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

    fn convert_query_join(def: &ast::QueryJoinDef) -> Result<QueryJoin, CompileError> {
        let formulas: Result<Vec<QueryFormula>, CompileError> = def
            .formulas
            .iter()
            .map(|f| {
                let expr = crate::expr::parse_expression(&f.expression, f.span.offset())?;
                Ok(QueryFormula {
                    name: f.name.as_str().into(),
                    expression: convert_expr(&expr),
                })
            })
            .collect();

        let joins: Result<Vec<QueryJoin>, CompileError> = def.joins.iter().map(convert_query_join).collect();

        Ok(QueryJoin {
            target_entity: def.target_entity.as_str().into(),
            selections: def
                .selections
                .iter()
                .map(|s| QuerySelection {
                    field: s.field.as_str().into(),
                    alias: s.alias.as_ref().map(|s| s.as_str().into()),
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
                    name: f.name.as_str().into(),
                    expression: convert_expr(&expr),
                })
            })
            .collect();

        let joins: Result<Vec<QueryJoin>, CompileError> = query_def.joins.iter().map(convert_query_join).collect();

        let filters: Result<Vec<gurih_ir::Expression>, CompileError> = query_def
            .filters
            .iter()
            .map(|f_str| {
                let expr = crate::expr::parse_expression(f_str, 0)?;
                Ok(convert_expr(&expr))
            })
            .collect();

        ir_queries.insert(
            query_def.name.as_str().into(),
            QuerySchema {
                name: query_def.name.as_str().into(),
                root_entity: query_def.root_entity.as_str().into(),
                query_type: query_def.query_type.clone(),
                selections: query_def
                    .selections
                    .iter()
                    .map(|s| QuerySelection {
                        field: s.field.as_str().into(),
                        alias: s.alias.as_ref().map(|s| s.as_str().into()),
                    })
                    .collect(),
                formulas: formulas?,
                filters: filters?,
                joins: joins?,
                group_by: query_def.group_by.iter().map(|s| Symbol::from(s.as_str())).collect(),
            },
        );
    }

    Ok(Schema {
        name: ast_root.name.unwrap_or("GurihApp".to_string()).as_str().into(),
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
    field_type: &FieldType,
    enums: &HashMap<String, Vec<Symbol>>,
    references: &Option<String>,
) -> Result<FieldType, CompileError> {
    match field_type {
        FieldType::Enum(_) => {
            // For explicit Enum type, references should be set to the enum name.
            let variants = if let Some(ref_name) = references {
                enums.get(ref_name).cloned().unwrap_or_default()
            } else {
                vec![]
            };
            Ok(FieldType::Enum(variants))
        }
        FieldType::Custom(s) => {
            // Check if it matches an enum name
            if let Some(variants) = enums.get(s) {
                Ok(FieldType::Enum(variants.clone()))
            } else {
                // If unknown, default to string
                Ok(FieldType::String)
            }
        }
        FieldType::Code => Ok(FieldType::Code),
        other => Ok(other.clone()),
    }
}
