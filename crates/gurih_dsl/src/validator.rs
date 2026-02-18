use crate::ast::{self, Ast};
use crate::diagnostics::SourceSpan;
use crate::errors::CompileError;
use crate::expr::{BinaryOpType, Expr, UnaryOpType};
use gurih_ir::FieldType;
use std::collections::{HashMap, HashSet};

#[derive(PartialEq, Debug, Clone, Copy)]
enum DataType {
    Number,
    String,
    Boolean,
    Any,
}

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
        self.validate_rules(ast)?;
        self.validate_workflows(ast)?;
        self.validate_employee_statuses(ast)?;
        Ok(())
    }

    fn get_entity_fields(&self, ast: &Ast) -> HashMap<String, HashSet<String>> {
        let mut entity_fields: HashMap<String, HashSet<String>> = HashMap::new();
        for entity in &ast.entities {
            let fields: HashSet<String> = entity.fields.iter().map(|f| f.name.clone()).collect();
            entity_fields.insert(entity.name.clone(), fields);
        }
        for module in &ast.modules {
            for entity in &module.entities {
                let fields: HashSet<String> = entity.fields.iter().map(|f| f.name.clone()).collect();
                entity_fields.insert(entity.name.clone(), fields);
            }
        }
        entity_fields
    }

    fn validate_workflows(&self, ast: &Ast) -> Result<(), CompileError> {
        // Build entity field map for validation
        let entity_fields = self.get_entity_fields(ast);

        for workflow in &ast.workflows {
            self.validate_entity_transitions_context(
                &workflow.entity,
                &workflow.transitions,
                &entity_fields,
                workflow.span,
                Some(&workflow.field),
            )?;
        }
        Ok(())
    }

    fn validate_employee_statuses(&self, ast: &Ast) -> Result<(), CompileError> {
        let entity_fields = self.get_entity_fields(ast);

        for status in &ast.employee_statuses {
            self.validate_entity_transitions_context(
                &status.entity,
                &status.transitions,
                &entity_fields,
                status.span,
                None,
            )?;
        }
        Ok(())
    }

    fn validate_entity_transitions_context(
        &self,
        entity_name: &str,
        transitions: &[ast::TransitionDef],
        entity_fields: &HashMap<String, HashSet<String>>,
        span: SourceSpan,
        required_field: Option<&str>,
    ) -> Result<(), CompileError> {
        if let Some(fields) = entity_fields.get(entity_name) {
            if let Some(field) = required_field
                && !fields.contains(field)
            {
                return Err(CompileError::ValidationError {
                    src: self.src.to_string(),
                    span,
                    message: format!("Target field '{}' not found in entity '{}'", field, entity_name),
                });
            }
            self.validate_transitions(transitions, fields, entity_name)?;
        } else {
            return Err(CompileError::ValidationError {
                src: self.src.to_string(),
                span,
                message: format!("Entity '{}' not found", entity_name),
            });
        }
        Ok(())
    }

    fn validate_transitions(
        &self,
        transitions: &[ast::TransitionDef],
        fields: &HashSet<String>,
        entity_name: &str,
    ) -> Result<(), CompileError> {
        for transition in transitions {
            for precondition in &transition.preconditions {
                if let ast::TransitionPreconditionDef::Assertion { expression, span } = precondition {
                    self.validate_expression_fields(expression, fields, *span)?;
                }
            }

            for effect in &transition.effects {
                #[allow(clippy::collapsible_if)]
                if let ast::TransitionEffectDef::UpdateField { field, span, .. } = effect {
                    if !fields.contains(field) {
                        return Err(CompileError::ValidationError {
                            src: self.src.to_string(),
                            span: *span,
                            message: format!("Effect target field '{}' not found in entity '{}'", field, entity_name),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_expression_fields(
        &self,
        expr: &Expr,
        fields: &HashSet<String>,
        span: SourceSpan,
    ) -> Result<(), CompileError> {
        match expr {
            Expr::Field(name, _) => {
                if !fields.contains(name) {
                    return Err(CompileError::ValidationError {
                        src: self.src.to_string(),
                        span,
                        message: format!("Field '{}' not found in entity", name),
                    });
                }
            }
            Expr::FunctionCall { args, .. } => {
                for arg in args {
                    self.validate_expression_fields(arg, fields, span)?;
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.validate_expression_fields(left, fields, span)?;
                self.validate_expression_fields(right, fields, span)?;
            }
            Expr::UnaryOp { expr, .. } => {
                self.validate_expression_fields(expr, fields, span)?;
            }
            Expr::Grouping(expr, _) => {
                self.validate_expression_fields(expr, fields, span)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn validate_rules(&self, ast: &Ast) -> Result<(), CompileError> {
        for rule in &ast.rules {
            let ty = self.infer_type(&rule.assertion)?;
            if ty != DataType::Boolean && ty != DataType::Any {
                return Err(CompileError::ValidationError {
                    src: self.src.to_string(),
                    span: rule.span,
                    message: format!("Rule assertion must evaluate to Boolean, found {:?}", ty),
                });
            }
        }
        Ok(())
    }

    fn infer_type(&self, expr: &Expr) -> Result<DataType, CompileError> {
        match expr {
            Expr::Literal(_, _) => Ok(DataType::Number),
            Expr::StringLiteral(_, _) => Ok(DataType::String),
            Expr::BoolLiteral(_, _) => Ok(DataType::Boolean),
            Expr::Field(_, _) => Ok(DataType::Any), // Cannot resolve types without full schema context yet
            Expr::FunctionCall { .. } => Ok(DataType::Any),
            Expr::UnaryOp { op, expr, span } => {
                let inner = self.infer_type(expr)?;
                match op {
                    UnaryOpType::Not => {
                        if inner != DataType::Boolean && inner != DataType::Any {
                            return Err(CompileError::ValidationError {
                                src: self.src.to_string(),
                                span: *span,
                                message: "NOT requires boolean".into(),
                            });
                        }
                        Ok(DataType::Boolean)
                    }
                    UnaryOpType::Neg => {
                        if inner != DataType::Number && inner != DataType::Any {
                            return Err(CompileError::ValidationError {
                                src: self.src.to_string(),
                                span: *span,
                                message: "Negation requires number".into(),
                            });
                        }
                        Ok(DataType::Number)
                    }
                }
            }
            Expr::BinaryOp { left, op, right, span } => {
                let l = self.infer_type(left)?;
                let r = self.infer_type(right)?;

                if l == DataType::Any || r == DataType::Any {
                    return match op {
                        BinaryOpType::Add | BinaryOpType::Sub | BinaryOpType::Mul | BinaryOpType::Div => {
                            Ok(DataType::Number)
                        }
                        _ => Ok(DataType::Boolean),
                    };
                }

                match op {
                    BinaryOpType::Add | BinaryOpType::Sub | BinaryOpType::Mul | BinaryOpType::Div => {
                        if l != DataType::Number || r != DataType::Number {
                            return Err(CompileError::ValidationError {
                                src: self.src.to_string(),
                                span: *span,
                                message: "Arithmetic requires numbers".into(),
                            });
                        }
                        Ok(DataType::Number)
                    }
                    BinaryOpType::And | BinaryOpType::Or => {
                        if l != DataType::Boolean || r != DataType::Boolean {
                            return Err(CompileError::ValidationError {
                                src: self.src.to_string(),
                                span: *span,
                                message: "Logic requires booleans".into(),
                            });
                        }
                        Ok(DataType::Boolean)
                    }
                    BinaryOpType::Gt | BinaryOpType::Lt | BinaryOpType::Gte | BinaryOpType::Lte => {
                        if l != r || l != DataType::Number {
                            return Err(CompileError::ValidationError {
                                src: self.src.to_string(),
                                span: *span,
                                message: "Comparison requires numbers".into(),
                            });
                        }
                        Ok(DataType::Boolean)
                    }
                    BinaryOpType::Eq | BinaryOpType::Neq => {
                        if l != r {
                            return Err(CompileError::ValidationError {
                                src: self.src.to_string(),
                                span: *span,
                                message: "Equality requires same types".into(),
                            });
                        }
                        Ok(DataType::Boolean)
                    }
                    BinaryOpType::Like | BinaryOpType::ILike => {
                        if l != DataType::String || r != DataType::String {
                            return Err(CompileError::ValidationError {
                                src: self.src.to_string(),
                                span: *span,
                                message: "Like requires strings".into(),
                            });
                        }
                        Ok(DataType::Boolean)
                    }
                }
            }
            Expr::Grouping(e, _) => self.infer_type(e),
        }
    }

    fn check_duplicate<F>(
        &self,
        names: &mut HashMap<String, SourceSpan>,
        name: &str,
        span: SourceSpan,
        message_fn: F,
    ) -> Result<(), CompileError>
    where
        F: FnOnce() -> String,
    {
        if names.contains_key(name) {
            return Err(CompileError::ValidationError {
                src: self.src.to_string(),
                span,
                message: message_fn(),
            });
        }
        names.insert(name.to_string(), span);
        Ok(())
    }

    fn validate_entities(&self, ast: &Ast) -> Result<(), CompileError> {
        let mut entity_names = HashMap::new();

        for module in &ast.modules {
            for entity in &module.entities {
                self.check_duplicate(&mut entity_names, &entity.name, entity.span, || {
                    format!("Duplicate entity name: {}", entity.name)
                })?;
                self.validate_entity(entity)?;
            }
        }

        for entity in &ast.entities {
            self.check_duplicate(&mut entity_names, &entity.name, entity.span, || {
                format!("Duplicate entity name: {}", entity.name)
            })?;
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
            self.check_duplicate(&mut field_names, &field.name, field.span, || {
                format!("Duplicate field name '{}' in entity '{}'", field.name, entity.name)
            })?;

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
                message: format!("Entity '{}' must have at least one primary key (field:pk)", entity.name),
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
                message: format!("Entity '{}' can have at most one field:description", entity.name),
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
            valid_targets.insert(page.name.as_str());
        }
        for dashboard in &ast.dashboards {
            valid_targets.insert(dashboard.name.as_str());
        }
        for action in &ast.actions {
            valid_targets.insert(action.name.as_str());
        }
        for module in &ast.modules {
            for action in &module.actions {
                valid_targets.insert(action.name.as_str());
            }
        }

        for routes in &ast.routes {
            for node in &routes.routes {
                self.validate_route_node(node, &valid_targets)?;
            }
        }

        Ok(())
    }

    fn validate_route_node(&self, node: &ast::RouteNode, valid_targets: &HashSet<&str>) -> Result<(), CompileError> {
        match node {
            ast::RouteNode::Route(r) => {
                if !valid_targets.contains(r.action.as_str()) {
                    return Err(CompileError::ValidationError {
                        src: self.src.to_string(),
                        span: r.span,
                        message: format!("Route target '{}' not found in pages, dashboards or actions", r.action),
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
