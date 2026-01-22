use crate::ast::{self, Ast};
use crate::errors::CompileError;
use gurih_ir::FieldType;
use std::collections::{HashMap, HashSet};

pub struct Validator<'a> {
    src: &'a str,
}

impl<'a> Validator<'a> {
    pub fn new(src: &'a str) -> Self {
        Self { src }
    }

    pub fn validate(&self, ast: &Ast) -> Result<(), CompileError> {
        self.validate_entities(ast)?;
        self.validate_routes(ast)?;
        Ok(())
    }

    fn validate_entities(&self, ast: &Ast) -> Result<(), CompileError> {
        let mut entity_names = HashMap::new();

        // Helper to check duplicates
        let mut check_duplicate = |name: &str, span| -> Result<(), CompileError> {
            if entity_names.contains_key(name) {
                return Err(CompileError::ValidationError {
                    src: self.src.to_string(),
                    span,
                    message: format!("Duplicate entity name: {}", name),
                });
            }
            entity_names.insert(name.to_string(), span);
            Ok(())
        };

        for module in &ast.modules {
            for entity in &module.entities {
                check_duplicate(&entity.name, entity.span)?;
                self.validate_entity(entity)?;
            }
        }

        for entity in &ast.entities {
            check_duplicate(&entity.name, entity.span)?;
            self.validate_entity(entity)?;
        }

        Ok(())
    }

    fn validate_entity(&self, entity: &ast::EntityDef) -> Result<(), CompileError> {
        let mut field_names = HashMap::new();
        let mut has_pk = false;
        let mut name_count = 0;
        let mut title_count = 0;
        let mut description_count = 0;
        let mut avatar_count = 0;

        for field in &entity.fields {
            if field_names.contains_key(&field.name) {
                return Err(CompileError::ValidationError {
                    src: self.src.to_string(),
                    span: field.span,
                    message: format!(
                        "Duplicate field name '{}' in entity '{}'",
                        field.name, entity.name
                    ),
                });
            }
            field_names.insert(field.name.clone(), field.span);

            // Note: We are checking raw AST types here.
            // Custom types or Enum are not fully resolved but we can check the variant.
            match &field.type_name {
                FieldType::Pk => has_pk = true,
                FieldType::Name => name_count += 1,
                FieldType::Title => title_count += 1,
                FieldType::Description => description_count += 1,
                FieldType::Avatar => avatar_count += 1,
                _ => {}
            }
        }

        if !has_pk {
            return Err(CompileError::ValidationError {
                src: self.src.to_string(),
                span: entity.span,
                message: format!(
                    "Entity '{}' must have at least one primary key (field:pk)",
                    entity.name
                ),
            });
        }

        if name_count > 1 {
            return Err(CompileError::ValidationError {
                src: self.src.to_string(),
                span: entity.span,
                message: format!("Entity '{}' can have at most one field:name", entity.name),
            });
        }
        if title_count > 1 {
            return Err(CompileError::ValidationError {
                src: self.src.to_string(),
                span: entity.span,
                message: format!("Entity '{}' can have at most one field:title", entity.name),
            });
        }
        if description_count > 1 {
            return Err(CompileError::ValidationError {
                src: self.src.to_string(),
                span: entity.span,
                message: format!(
                    "Entity '{}' can have at most one field:description",
                    entity.name
                ),
            });
        }
        if avatar_count > 1 {
            return Err(CompileError::ValidationError {
                src: self.src.to_string(),
                span: entity.span,
                message: format!("Entity '{}' can have at most one field:avatar", entity.name),
            });
        }

        Ok(())
    }

    fn validate_routes(&self, ast: &Ast) -> Result<(), CompileError> {
        // Collect valid targets
        let mut valid_targets = HashSet::new();

        for page in &ast.pages {
            valid_targets.insert(page.name.clone());
        }
        for dashboard in &ast.dashboards {
            valid_targets.insert(dashboard.name.clone());
        }
        for action in &ast.actions {
            valid_targets.insert(action.name.clone());
        }
        for module in &ast.modules {
            for action in &module.actions {
                valid_targets.insert(action.name.clone());
            }
        }

        for routes in &ast.routes {
            for node in &routes.routes {
                self.validate_route_node(node, &valid_targets)?;
            }
        }

        Ok(())
    }

    fn validate_route_node(
        &self,
        node: &ast::RouteNode,
        valid_targets: &HashSet<String>,
    ) -> Result<(), CompileError> {
        match node {
            ast::RouteNode::Route(r) => {
                if !valid_targets.contains(&r.action) {
                    return Err(CompileError::ValidationError {
                        src: self.src.to_string(),
                        span: r.span,
                        message: format!(
                            "Route target '{}' not found in pages, dashboards or actions",
                            r.action
                        ),
                    });
                }
            }
            ast::RouteNode::Group(g) => {
                for child in &g.children {
                    self.validate_route_node(child, valid_targets)?;
                }
            }
        }
        Ok(())
    }
}
