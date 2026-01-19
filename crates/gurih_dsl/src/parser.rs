use kdl::{KdlDocument, KdlNode};
use crate::ast::*;
use crate::errors::CompileError;

pub fn parse(src: &str) -> Result<Ast, CompileError> {
    let doc: KdlDocument = src.parse()?;
    let mut ast = Ast {
        name: None,
        version: None,
        modules: vec![],
        entities: vec![],
        workflows: vec![],
        forms: vec![],
        permissions: vec![],
    };

    for node in doc.nodes() {
        let name = node.name().value();
        match name {
            "name" => ast.name = Some(get_arg_string(node, 0, src)?),
            "version" => ast.version = Some(get_arg_string(node, 0, src)?),
            "database" => {}, // Ignore database for now
            "module" => ast.modules.push(parse_module(node, src)?),
            "entity" => ast.entities.push(parse_entity(node, src)?),
            "workflow" => ast.workflows.push(parse_workflow(node, src)?),
            "form" => ast.forms.push(parse_form(node, src)?),
            "permission" => ast.permissions.push(parse_permission(node, src)?),
            _ => {
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
        span: node.span().clone(),
    })
}

fn parse_entity(node: &KdlNode, src: &str) -> Result<EntityDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let mut fields = vec![];

    if let Some(children) = node.children() {
        for child in children.nodes() {
            let child_name = child.name().value();
            match child_name {
                "field" => fields.push(parse_field(child, src)?),
                // Shorthands
                "id" => {
                     fields.push(FieldDef {
                        name: "id".to_string(),
                        type_name: "Integer".to_string(), // Default ID type
                        required: true,
                        unique: true,
                        references: None,
                        span: child.span().clone(), 
                     });
                }
                "string" | "text" | "integer" | "float" | "boolean" | "date" | "datetime" => {
                     let field_name = get_arg_string(child, 0, src)?;
                     let type_name = capitalize(child_name);
                     let required = get_prop_bool(child, "required").unwrap_or(false);
                     let unique = get_prop_bool(child, "unique").unwrap_or(false);
                     
                     fields.push(FieldDef {
                        name: field_name,
                        type_name,
                        required,
                        unique,
                        references: None,
                        span: child.span().clone(),
                     });
                }
                "ref" => {
                    let field_name = get_arg_string(child, 0, src)?;
                    let target = get_arg_string(child, 1, src)?; // second arg is target e.g. "Department.id"
                    
                    fields.push(FieldDef {
                        name: field_name,
                        type_name: "Relation".to_string(),
                        required: get_prop_bool(child, "required").unwrap_or(false),
                        unique: false,
                        references: Some(target), // We'll parse "Entity.id" later or just store it
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
        span: node.span().clone(),
    })
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn parse_field(node: &KdlNode, src: &str) -> Result<FieldDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let type_name = get_prop_string(node, "type", src)?;
    
    let required = get_prop_bool(node, "required").unwrap_or(false);
    let unique = get_prop_bool(node, "unique").unwrap_or(false);
    let references = get_prop_string(node, "references", src).ok();

    Ok(FieldDef {
        name,
        type_name,
        required,
        unique,
        references,
        span: node.span().clone(),
    })
}

fn parse_workflow(node: &KdlNode, src: &str) -> Result<WorkflowDef, CompileError> {
    let name = get_arg_string(node, 0, src)?;
    let entity = get_prop_string(node, "entity", src)?;
    
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
    let name = get_arg_string(node, 0, src)?;
    let entity = get_prop_string(node, "entity", src)?;
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
    let name = get_arg_string(node, 0, src)?;
    let mut rules = vec![];
    
    // assume rules are just args or properties for now, or children?
    // Let's assume children: rule "resource:action"
    if let Some(children) = node.children() {
        for child in children.nodes() {
            if child.name().value() == "rule" {
                let rule = get_arg_string(child, 0, src)?;
                rules.push(rule);
            }
        }
    }

    Ok(PermissionDef {
        name,
        rules,
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
            message: format!("Missing or invalid argument at index {}", index),
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
