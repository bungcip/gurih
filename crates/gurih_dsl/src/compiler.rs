use crate::errors::CompileError;
use crate::parser::parse;
use gurih_ir::{Schema, EntitySchema, FieldSchema, FieldType, WorkflowSchema, FormSchema, FormSection, PermissionSchema, Transition};
use std::collections::HashMap;

pub fn compile(src: &str) -> Result<Schema, CompileError> {
    let ast = parse(src)?;
    
    // Validation Context
    let mut entity_names = HashMap::new();
    
    let mut ir_entities = HashMap::new();
    let mut ir_workflows = HashMap::new();
    let mut ir_forms = HashMap::new();
    let mut ir_permissions = HashMap::new();

    // 1. Process Entities
    for entity_def in &ast.entities {
        if entity_names.contains_key(&entity_def.name) {
             return Err(CompileError::ValidationError {
                src: src.to_string(),
                span: entity_def.span.clone(),
                message: format!("Duplicate entity name: {}", entity_def.name),
            });
        }
        entity_names.insert(entity_def.name.clone(), entity_def.span.clone());
        
        let mut fields = vec![];
        for field_def in &entity_def.fields {
            let field_type = parse_field_type(&field_def.type_name, src, &field_def.span)?;
            fields.push(FieldSchema {
                name: field_def.name.clone(),
                field_type,
                required: field_def.required,
                unique: field_def.unique,
                default: None,
                references: field_def.references.clone(),
            });
        }
        
        ir_entities.insert(entity_def.name.clone(), EntitySchema {
            name: entity_def.name.clone(),
            fields,
        });
    }

    // 2. Process Workflows
    for wf_def in &ast.workflows {
        if !entity_names.contains_key(&wf_def.entity) {
             return Err(CompileError::ValidationError {
                src: src.to_string(),
                span: wf_def.span.clone(),
                message: format!("Workflow references unknown entity: {}", wf_def.entity),
            });
        }
        
        let state_names: Vec<String> = wf_def.states.iter().map(|s| s.name.clone()).collect();
        
        // Validate transitions
        for t in &wf_def.transitions {
            if !state_names.contains(&t.from) {
                return Err(CompileError::ValidationError {
                    src: src.to_string(),
                    span: t.span.clone(),
                    message: format!("Transition 'from' state unknown: {}", t.from),
                });
            }
             if !state_names.contains(&t.to) {
                return Err(CompileError::ValidationError {
                    src: src.to_string(),
                    span: t.span.clone(),
                    message: format!("Transition 'to' state unknown: {}", t.to),
                });
            }
        }
        
        let initial_state = wf_def.states.iter().find(|s| s.initial).map(|s| s.name.clone())
            .ok_or_else(|| CompileError::ValidationError {
                 src: src.to_string(),
                 span: wf_def.span.clone(),
                 message: "Workflow must have an initial state".to_string(),
            })?;

        ir_workflows.insert(wf_def.name.clone(), WorkflowSchema {
            name: wf_def.name.clone(),
            entity: wf_def.entity.clone(),
            initial_state,
            states: state_names,
            transitions: wf_def.transitions.iter().map(|t| Transition {
                name: t.name.clone(),
                from: t.from.clone(),
                to: t.to.clone(),
                required_permission: t.permission.clone(),
            }).collect(),
        });
    }
    
    // 3. Process Forms
    for form_def in &ast.forms {
        // Validate entity exists
         if !entity_names.contains_key(&form_def.entity) {
             return Err(CompileError::ValidationError {
                src: src.to_string(),
                span: form_def.span.clone(),
                message: format!("Form references unknown entity: {}", form_def.entity),
            });
        }
        
        ir_forms.insert(form_def.name.clone(), FormSchema {
            name: form_def.name.clone(),
            entity: form_def.entity.clone(),
            sections: form_def.sections.iter().map(|s| FormSection {
                title: s.title.clone(),
                fields: s.fields.clone(),
            }).collect(),
        });
    }

    // 4. Process Permissions
    for perm_def in &ast.permissions {
        ir_permissions.insert(perm_def.name.clone(), PermissionSchema {
            name: perm_def.name.clone(),
            rules: perm_def.rules.clone(),
        });
    }

    Ok(Schema {
        version: "1.0".to_string(),
        entities: ir_entities,
        workflows: ir_workflows,
        forms: ir_forms,
        permissions: ir_permissions,
    })
}

fn parse_field_type(type_name: &str, src: &str, span: &miette::SourceSpan) -> Result<FieldType, CompileError> {
    match type_name {
        "String" => Ok(FieldType::String),
        "Text" => Ok(FieldType::Text),
        "Integer" => Ok(FieldType::Integer),
        "Float" => Ok(FieldType::Float),
        "Boolean" => Ok(FieldType::Boolean),
        "Date" => Ok(FieldType::Date),
        "DateTime" => Ok(FieldType::DateTime),
        "Relation" => Ok(FieldType::Relation),
        _ => Err(CompileError::ValidationError {
            src: src.to_string(),
            span: span.clone(),
            message: format!("Unknown field type: {}", type_name),
        })
    }
}
