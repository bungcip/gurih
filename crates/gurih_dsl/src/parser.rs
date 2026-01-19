use kdl::{KdlDocument, KdlNode, KdlValue};
use crate::ast::*;
use crate::errors::CompileError;

pub fn parse(src: &str) -> Result<Ast, CompileError> {
    let doc: KdlDocument = src.parse()?;
    let mut ast = Ast {
        name: None,
        version: None,
        database: None,
        icons: vec![],
        layouts: vec![],
        modules: vec![],
        entities: vec![],
        enums: vec![],
        serials: vec![],
        workflows: vec![],
        dashboards: vec![],
        pages: vec![],
        routes: vec![],
        menus: vec![],
        prints: vec![],
        permissions: vec![],
    };

    for node in doc.nodes() {
        let name = node.name().value();
        match name {
            "name" => ast.name = Some(get_arg_string(node, 0, src)?),
            "version" => ast.version = Some(get_arg_string(node, 0, src)?),
            "database" => ast.database = Some(parse_database(node, src)?),
            "icons" => ast.icons.extend(parse_icons(node, src)?),
            "layout" => ast.layouts.push(parse_layout(node, src)?),
            "module" => ast.modules.push(parse_module(node, src)?),
            "entity" => ast.entities.push(parse_entity(node, src)?),
            "enum" => ast.enums.push(parse_enum(node, src)?),
            "serial" => ast.serials.push(parse_serial(node, src)?),
            "workflow" => ast.workflows.push(parse_workflow(node, src)?),
            "dashboard" => ast.dashboards.push(parse_dashboard(node, src)?),
            "page" => ast.pages.push(parse_page(node, src)?),
            "routes" => ast.routes.push(parse_routes(node, src)?),
            "menu" => ast.menus.push(parse_menu(node, src)?),
            "print" => ast.prints.push(parse_print(node, src)?),
            "role" | "permission" => ast.permissions.push(parse_permission(node, src)?),
            _ => {
                // Ignore unknown nodes or warn? Strict for now.
                 return Err(CompileError::ParseError {
                    src: src.to_string(),
                    span: node.name().span().clone(),
                    message: format!("Unknown top-level definition: {}", name),
                });
            }
        }
    }

    Ok(ast)
}

fn parse_database(node: &KdlNode, src: &str) -> Result<DatabaseDef, CompileError> {
    let mut db_type = String::new();
    let mut url = String::new();

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "type" => db_type = get_arg_string(child, 0, src)?,
                "url" => url = get_arg_string(child, 0, src)?,
                _ => {}
            }
        }
    }

    Ok(DatabaseDef {
        db_type,
        url,
        span: node.span().clone(),
    })
}

fn parse_icons(node: &KdlNode, src: &str) -> Result<Vec<IconDef>, CompileError> {
    let mut icons = vec![];
    if let Some(children) = node.children() {
        for child in children.nodes() {
            // Icon alias is the node name if quoted? KDL node names can be strings.
            // But usually KDL is `node "arg"`. DSL ex: `"trash" "lucide:trash"`
            // Wait, standard KDL `trash "lucide:trash"` is node name "trash".
            // DSL ex says: `"trash" "lucide:trash-2"`.
            // If the node name is "trash", then arg 0 is "lucide:trash-2".
            
            let name = child.name().value().to_string();
            let uri = get_arg_string(child, 0, src)?;
            icons.push(IconDef {
                name,
                uri,
                span: child.span().clone(),
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
        span: node.span().clone(),
    })
}

fn parse_layout_section(node: &KdlNode, src: &str) -> Result<LayoutSectionDef, CompileError> {
    // Check if simple `header false` or block `header { ... }`
    // If it has args and no children, treat as bool/string?
    // DSL ex: `header false` or `header { search_bar true }`
    
    let enabled = if let Ok(val) = get_arg_bool(node, 0) {
        val
    } else {
        true // default true if block exists
    };

    let mut props = std::collections::HashMap::new();
    let mut menu_ref = None;

    if enabled {
        if let Some(children) = node.children() {
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
    }

    Ok(LayoutSectionDef {
        enabled,
        props,
        menu_ref,
        span: node.span().clone(),
    })
}

fn parse_module(node: &KdlNode, src: &str) -> Result<ModuleDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut entities = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            match child.name().value() {
                "entity" => entities.push(parse_entity(child, src)?),
                _ => {} // Ignore other children for now
            }
        }
    }

    Ok(ModuleDef {
        name,
        entities,
        enums: vec![],
        span: node.span().clone(),
    })
}

fn parse_entity(node: &KdlNode, src: &str) -> Result<EntityDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut fields = vec![];
    let mut relationships = vec![];
    let mut options = EntityOptions::default();

    if let Some(children) = node.children() {
        for child in children.nodes() {
            let child_name = child.name().value();
            match child_name {
                "field" => fields.push(parse_field(child, src)?),
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
                "belongs_to" | "has_many" | "has_one" => {
                    let rel_type = match child_name {
                        "belongs_to" => RelationshipType::BelongsTo,
                        "has_many" => RelationshipType::HasMany,
                        "has_one" => RelationshipType::HasOne,
                        _ => unreachable!(),
                    };
                    
                    // has_many "orders" "Order"
                    // belongs_to "Department" (implies name="department", target="Department")
                    
                    let arg0 = get_arg_string(child, 0, src)?;
                    let (name, target) = if let Ok(arg1) = get_arg_string(child, 1, src) {
                        (arg0, arg1) // name, target
                    } else {
                        // Infer name from target
                        // e.g. target="Department" -> name="department"
                        (arg0.to_lowercase(), arg0.clone())
                    };
                    
                    relationships.push(RelationshipDef {
                        rel_type,
                        name,
                        target_entity: target,
                        span: child.span().clone(),
                    });
                }
                
                // Semantic types shorthand
                "id" => {
                     fields.push(FieldDef {
                        name: "id".to_string(),
                        type_name: "Integer".to_string(),
                        serial: None,
                        required: true,
                        unique: true,
                        default: None,
                        references: None,
                        span: child.span().clone(), 
                     });
                }
                "string" | "text" | "int" | "integer" | "float" | "decimal" | "bool" | "boolean" | "date" | "datetime" | "time" | "money" | "code" | "enum" | "name" | "email" | "phone" | "description" => {
                    // code "field_name" generator="GenName"
                    // enum "status" "StatusEnum" default="Draft"
                    
                     let type_name = capitalize(child_name);
                     let field_name = get_arg_string(child, 0, src).unwrap_or(child_name.to_string());
                     
                     let required = get_prop_bool(child, "required").unwrap_or(false);
                     let unique = get_prop_bool(child, "unique").unwrap_or(false);
                     let default = get_prop_string(child, "default", src).ok();
                     let serial = get_prop_string(child, "serial", src).ok();
                     
                     // For Enum, the second arg is the enum name
                     let references = if child_name == "enum" {
                        Some(get_arg_string(child, 1, src)?)
                     } else {
                        get_prop_string(child, "references", src).ok()
                     };

                     fields.push(FieldDef {
                        name: field_name,
                        type_name,
                        serial,
                        required,
                        unique,
                        default,
                        references,
                        span: child.span().clone(),
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
        span: node.span().clone(),
    })
}

fn parse_field(node: &KdlNode, src: &str) -> Result<FieldDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let type_name = get_prop_string(node, "type", src)?;
    
    let required = get_prop_bool(node, "required").unwrap_or(false);
    let unique = get_prop_bool(node, "unique").unwrap_or(false);
    let references = get_prop_string(node, "references", src).ok();
    let default = get_prop_string(node, "default", src).ok();
    let serial = get_prop_string(node, "serial", src).ok();

    Ok(FieldDef {
        name,
        type_name,
        serial,
        required,
        unique,
        default,
        references,
        span: node.span().clone(),
    })
}

fn parse_workflow(node: &KdlNode, src: &str) -> Result<WorkflowDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    // support 'for' or 'entity'
    let entity = get_prop_string(node, "for", src)
        .or_else(|_| get_prop_string(node, "entity", src))?;
        
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
        span: node.span().clone(),
    })
}

fn parse_state(node: &KdlNode, src: &str) -> Result<StateDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let initial = get_prop_bool(node, "initial").unwrap_or(false);

    Ok(StateDef {
        name,
        initial,
        span: node.span().clone(),
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
        span: node.span().clone(),
    })
}

fn parse_form(node: &KdlNode, src: &str) -> Result<FormDef, CompileError> {
    let name = get_arg_string(node, 0, src).unwrap_or_else(|_| "DefaultForm".to_string());
    let entity = get_prop_string(node, "entity", src)
        .or_else(|_| get_prop_string(node, "for", src))?;
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
        span: node.span().clone(),
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
            } else if child.name().value() == "group" {
                if let Some(grandkids) = child.children() {
                    for grandkid in grandkids.nodes() {
                        if grandkid.name().value() == "field" {
                            let field_name = get_arg_string(grandkid, 0, src)?;
                            fields.push(field_name);
                        }
                    }
                }
            }
        }
    }

    Ok(FormSectionDef {
        title,
        fields,
        span: node.span().clone(),
    })
}

fn parse_permission(node: &KdlNode, src: &str) -> Result<PermissionDef, CompileError> {
    let name = get_arg_string(node, 0, src)?; // role "Admin"
    let mut allows = vec![];
    
    if let Some(children) = node.children() {
        for child in children.nodes() {
            if child.name().value() == "allow" {
                // allow "resource" "action"
                let resource = get_arg_string(child, 0, src)?;
                let actions = get_arg_string(child, 1, src).ok();
                
                allows.push(AllowDef {
                    resource,
                    actions,
                });
            }
        }
    }

    Ok(PermissionDef {
        name,
        allows,
        span: node.span().clone(),
    })
}

fn parse_enum(node: &KdlNode, src: &str) -> Result<EnumDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut variants = vec![];
    
    if let Some(children) = node.children() {
        for child in children.nodes() {
            // Enum variants are simple nodes, e.g. `Draft`
            variants.push(child.name().value().to_string());
        }
    }

    Ok(EnumDef {
        name,
        variants,
        span: node.span().clone(),
    })
}

fn parse_serial(node: &KdlNode, src: &str) -> Result<SerialDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut prefix = None;
    let mut date_format = None;
    let mut sequence_digits = 4; // default

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

    Ok(SerialDef {
        name,
        prefix,
        date_format,
        sequence_digits,
        span: node.span().clone(),
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
                    // Flatten widgets for now, recursively
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
        span: node.span().clone(),
    })
}

fn parse_widget(node: &KdlNode, src: &str) -> Result<WidgetDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let widget_type = get_prop_string(node, "type", src)?;
    
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
        span: node.span().clone(),
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
        span: node.span().clone(),
    })
}

fn parse_datatable(node: &KdlNode, src: &str) -> Result<DatatableDef, CompileError> {
    let entity = get_prop_string(node, "for", src)?;
    let mut columns = vec![];
    let mut actions = vec![];
    
    if let Some(children) = node.children() {
        for child in children.nodes() {
             match child.name().value() {
                 "column" => {
                     let field = get_arg_string(child, 0, src)?;
                     let label = get_prop_string(child, "label", src).unwrap_or_else(|_| capitalize(&field));
                     columns.push(DatatableColumnDef { field, label });
                 }
                 "action" => {
                     let label = get_arg_string(child, 0, src)?;
                     let icon = get_prop_string(child, "icon", src).ok();
                     let to = get_prop_string(child, "to", src).ok();
                     let variant = get_prop_string(child, "variant", src).ok();
                     
                     actions.push(ActionDef { label, icon, to, variant });
                 }
                 _ => {}
             }
        }
    }

    Ok(DatatableDef {
        entity,
        columns,
        actions,
        span: node.span().clone(),
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
        span: node.span().clone(),
    })
}

fn parse_route_node(node: &KdlNode, src: &str) -> Result<RouteNode, CompileError> {
    match node.name().value() {
        "route" => {
            let path = get_arg_string(node, 0, src)?;
            let to = get_prop_string(node, "to", src)?;
            let layout = get_prop_string(node, "layout", src).ok();
            let permission = get_prop_string(node, "permission", src).ok();
            
            Ok(RouteNode::Route(RouteDef {
                path,
                to,
                layout,
                permission,
                span: node.span().clone(),
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
                span: node.span().clone(),
            }))
        }
        _ => Err(CompileError::ParseError {
             src: src.to_string(),
             span: node.span().clone(),
             message: format!("Unexpected node in routes: {}", node.name().value()),
        })
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
        span: node.span().clone(),
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
                span: node.span().clone(),
            }))
        },
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
                span: node.span().clone(),
            }))
        },
        _ => Err(CompileError::ParseError {
             src: src.to_string(),
             span: node.span().clone(),
             message: format!("Unexpected node in menu: {}", name),
        })
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
         span: node.span().clone(),
     })
}


// Helpers

fn get_arg_string(node: &KdlNode, index: usize, src: &str) -> Result<String, CompileError> {
    node.entry(index)
        .and_then(|val| val.value().as_string().map(|s| s.to_string()))
        .ok_or_else(|| CompileError::ParseError {
            src: src.to_string(),
            span: node.span().clone(),
            message: format!("Missing or invalid argument at index {} for node '{}'", index, node.name().value()),
        })
}

fn get_prop_string(node: &KdlNode, key: &str, src: &str) -> Result<String, CompileError> {
    node.get(key)
        .and_then(|val| val.as_string().map(|s| s.to_string()))
        .ok_or_else(|| CompileError::ParseError {
            src: src.to_string(),
            span: node.span().clone(),
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
            span: node.span().clone(),
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
            span: node.span().clone(),
            message: format!("Missing or invalid bool argument at index {}", index),
        })
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
