use crate::store::validate_identifier;
use gurih_ir::{BinaryOperator, DatabaseType, Expression, QueryJoin, Schema, UnaryOperator};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::cell::RefCell;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryPlan {
    ExecuteSql {
        sql: String,
        params: Vec<Value>,
    },
    ExecuteHierarchy {
        sql: String,
        params: Vec<Value>,
        parent_field: String,
        rollup_fields: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueryExecutionStrategy {
    pub plans: Vec<QueryPlan>,
}

struct QueryBuilderState<'a> {
    schema: &'a Schema,
    select_parts: &'a mut Vec<String>,
    join_parts: &'a mut Vec<String>,
    params: &'a mut Vec<Value>,
    db_type: &'a DatabaseType,
    runtime_params: &'a std::collections::HashMap<String, Value>,
}

pub struct QueryEngine;

impl QueryEngine {
    pub fn plan(
        schema: &Schema,
        query_name: &str,
        runtime_params: &std::collections::HashMap<String, Value>,
    ) -> Result<QueryExecutionStrategy, String> {
        let query = schema
            .queries
            .get(&query_name.into())
            .ok_or_else(|| format!("Query '{}' not found in schema", query_name))?;

        let db_type = schema
            .database
            .as_ref()
            .map(|d| d.db_type.clone())
            .unwrap_or(DatabaseType::Sqlite);

        let mut select_parts = vec![];
        let mut join_parts = vec![];
        let mut params = vec![];
        let root_table = Self::entity_to_table(&query.root_entity.to_string());
        validate_identifier(&root_table)?;

        // Process Root Selections & Formulas
        for sel in &query.selections {
            let col_sql = format!("{}.{}", root_table, sel.field);
            if let Some(alias) = &sel.alias {
                select_parts.push(format!("{} AS {}", col_sql, alias));
            } else {
                select_parts.push(col_sql);
            }
        }
        for form in &query.formulas {
            let expr_sql = Self::expression_to_sql(&form.expression, &mut params, &db_type, runtime_params);
            select_parts.push(format!("{} AS {}", expr_sql, form.name));
        }

        // Process Joins (Recursive)
        let mut state = QueryBuilderState {
            schema,
            select_parts: &mut select_parts,
            join_parts: &mut join_parts,
            params: &mut params,
            db_type: &db_type,
            runtime_params,
        };

        Self::process_joins(&query.joins, &root_table, &query.root_entity.to_string(), &mut state)?;

        let select_clause = if select_parts.is_empty() {
            "*".to_string()
        } else {
            select_parts.join(", ")
        };

        let join_clause = join_parts.join(" ");

        let mut where_clause = String::new();
        if !query.filters.is_empty() {
            let filter_parts: Vec<String> = query
                .filters
                .iter()
                .map(|e| Self::expression_to_sql(e, &mut params, &db_type, runtime_params))
                .collect();
            where_clause = format!("WHERE {}", filter_parts.join(" AND "));
        }

        let mut group_by_clause = String::new();
        if !query.group_by.is_empty() {
            for s in &query.group_by {
                validate_identifier(s.as_str())?;
            }
            // Naive field formatting, assumes fields are columns of root or joined tables.
            // But symbols don't have table qualification.
            // We might need to assume root table or use qualified names in DSL.
            // For now, let's wrap in brackets.
            // Actually, expression_to_sql handles Field as [name].
            let group_parts: Vec<String> = query.group_by.iter().map(|s| format!("[{}]", s)).collect();
            group_by_clause = format!("GROUP BY {}", group_parts.join(", "));
        }

        let sql = format!(
            "SELECT {} FROM {} {} {} {}",
            select_clause, root_table, join_clause, where_clause, group_by_clause
        )
        .trim()
        .to_string();

        if query.query_type == gurih_ir::QueryType::Hierarchy {
            let h = query
                .hierarchy
                .as_ref()
                .ok_or("Hierarchy definition missing for query:hierarchy")?;
            Ok(QueryExecutionStrategy {
                plans: vec![QueryPlan::ExecuteHierarchy {
                    sql,
                    params,
                    parent_field: h.parent_field.to_string(),
                    rollup_fields: h.rollup_fields.iter().map(|s| s.to_string()).collect(),
                }],
            })
        } else {
            Ok(QueryExecutionStrategy {
                plans: vec![QueryPlan::ExecuteSql { sql, params }],
            })
        }
    }

    fn process_joins(
        joins: &[QueryJoin],
        parent_table: &str,
        parent_entity: &str,
        state: &mut QueryBuilderState,
    ) -> Result<(), String> {
        for join in joins {
            let target_entity_name = &join.target_entity;
            let target_table = Self::entity_to_table(&target_entity_name.to_string());
            validate_identifier(&target_table)?;

            // Find relationship to determine join condition
            // Assuming parent has relationship to target or vice versa
            let mut join_condition = String::new();

            // Attempts to determine join condition from schema
            if let Some(parent_ent) = state.schema.entities.get(&parent_entity.into())
                && let Some(rel) = parent_ent
                    .relationships
                    .iter()
                    .find(|r| r.target_entity == *target_entity_name)
            {
                if rel.rel_type == gurih_ir::RelationshipType::BelongsTo {
                    // Parent has FK: parent.rel_id = target.id
                    join_condition = format!("{}.{}_id = {}.id", parent_table, rel.name, target_table);
                } else {
                    // Target has FK: target.parent_id = parent.id
                    // Assuming standard naming convention for back-ref
                    join_condition = format!("{}.{}_id = {}.id", target_table, parent_table, parent_table);
                }
            }

            // Fallback: Default heuristic (HasMany style)
            if join_condition.is_empty() {
                join_condition = format!("{}.{}_id = {}.id", target_table, parent_table, parent_table);
            }

            state
                .join_parts
                .push(format!("LEFT JOIN {} ON {}", target_table, join_condition));

            for sel in &join.selections {
                let col_sql = format!("{}.{}", target_table, sel.field);
                if let Some(alias) = &sel.alias {
                    state.select_parts.push(format!("{} AS {}", col_sql, alias));
                } else {
                    state.select_parts.push(col_sql);
                }
            }
            for form in &join.formulas {
                let expr_sql =
                    Self::expression_to_sql(&form.expression, state.params, state.db_type, state.runtime_params);
                state.select_parts.push(format!("{} AS {}", expr_sql, form.name));
            }

            Self::process_joins(&join.joins, &target_table, &target_entity_name.to_string(), state)?;
        }
        Ok(())
    }

    fn entity_to_table(s: &str) -> Arc<str> {
        thread_local! {
            static CACHE: RefCell<HashMap<String, Arc<str>>> = RefCell::new(HashMap::new());
        }
        CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            if let Some(val) = cache.get(s) {
                return val.clone();
            }

            let mut result = String::new();
            for (i, c) in s.char_indices() {
                if c.is_uppercase() {
                    if i > 0 {
                        result.push('_');
                    }
                    result.push(c.to_ascii_lowercase());
                } else {
                    result.push(c);
                }
            }

            let val: Arc<str> = Arc::from(result);
            if s.len() <= 128 {
                if cache.len() > 1000 {
                    cache.clear();
                }
                cache.insert(s.to_string(), val.clone());
            }
            val
        })
    }

    fn expression_to_sql(
        expr: &Expression,
        params: &mut Vec<Value>,
        db_type: &DatabaseType,
        runtime_params: &std::collections::HashMap<String, Value>,
    ) -> String {
        match expr {
            Expression::Field(f) => {
                let s = f.as_str();
                if s.contains('.') {
                    s.split('.')
                        .map(|part| format!("[{}]", part))
                        .collect::<Vec<_>>()
                        .join(".")
                } else {
                    format!("[{}]", f)
                }
            }
            Expression::Literal(n) => n.to_string(),
            Expression::BoolLiteral(b) => {
                if *b {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            Expression::StringLiteral(s) => {
                params.push(Value::String(s.clone()));
                if *db_type == DatabaseType::Postgres {
                    format!("${}", params.len())
                } else {
                    "?".to_string()
                }
            }
            Expression::FunctionCall { name, args } => {
                if name.as_str() == "param" {
                    if let Some(Expression::StringLiteral(key)) = args.first()
                        && let Some(val) = runtime_params.get(key)
                    {
                        params.push(val.clone());
                        if *db_type == DatabaseType::Postgres {
                            return format!("${}", params.len());
                        } else {
                            return "?".to_string();
                        }
                    }
                    return "NULL".to_string();
                } else if name.as_str() == "if" {
                    if args.len() == 3 {
                        let cond = Self::expression_to_sql(&args[0], params, db_type, runtime_params);
                        let true_val = Self::expression_to_sql(&args[1], params, db_type, runtime_params);
                        let false_val = Self::expression_to_sql(&args[2], params, db_type, runtime_params);
                        return format!("CASE WHEN {} THEN {} ELSE {} END", cond, true_val, false_val);
                    } else {
                        return "NULL".to_string();
                    }
                } else if name.as_str() == "running_sum" {
                    // running_sum(expr, partition_by, order_by)
                    if args.len() >= 3 {
                        let expr = Self::expression_to_sql(&args[0], params, db_type, runtime_params);
                        let partition = Self::expression_to_sql(&args[1], params, db_type, runtime_params);
                        let order = Self::expression_to_sql(&args[2], params, db_type, runtime_params);
                        return format!(
                            "SUM({}) OVER (PARTITION BY {} ORDER BY {} ROWS UNBOUNDED PRECEDING)",
                            expr, partition, order
                        );
                    }
                }

                let args_sql: Vec<String> = args
                    .iter()
                    .map(|a| Self::expression_to_sql(a, params, db_type, runtime_params))
                    .collect();
                format!("{}({})", name, args_sql.join(", "))
            }
            Expression::UnaryOp { op, expr } => {
                let expr_sql = Self::expression_to_sql(expr, params, db_type, runtime_params);
                match op {
                    UnaryOperator::Not => format!("NOT ({})", expr_sql),
                    UnaryOperator::Neg => format!("-({})", expr_sql),
                }
            }
            Expression::BinaryOp { left, op, right } => {
                let op_str = match op {
                    BinaryOperator::Add => "+",
                    BinaryOperator::Sub => "-",
                    BinaryOperator::Mul => "*",
                    BinaryOperator::Div => "/",
                    BinaryOperator::Eq => "=",
                    BinaryOperator::Neq => "<>",
                    BinaryOperator::Gt => ">",
                    BinaryOperator::Lt => "<",
                    BinaryOperator::Gte => ">=",
                    BinaryOperator::Lte => "<=",
                    BinaryOperator::And => "AND",
                    BinaryOperator::Or => "OR",
                };
                format!(
                    "{} {} {}",
                    Self::expression_to_sql(left, params, db_type, runtime_params),
                    op_str,
                    Self::expression_to_sql(right, params, db_type, runtime_params)
                )
            }
            Expression::Grouping(inner) => {
                format!("({})", Self::expression_to_sql(inner, params, db_type, runtime_params))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gurih_ir::{BinaryOperator, Expression, QueryFormula, QueryJoin, QuerySchema, QuerySelection, QueryType};

    #[test]
    fn test_lower_perspective_query() {
        let mut schema = Schema::default();

        // Setup Query Schema
        let query = QuerySchema {
            name: "ActiveCourseQuery".into(),
            params: vec![],
            root_entity: "CourseEntity".into(),
            query_type: QueryType::Nested,
            filters: vec![],
            group_by: vec![],
            selections: vec![QuerySelection {
                field: "title".into(),
                alias: None,
            }],
            formulas: vec![QueryFormula {
                name: "total_duration".into(),
                expression: Expression::FunctionCall {
                    name: "SUM".into(),
                    args: vec![Expression::Field("duration".into())],
                },
            }],
            joins: vec![QueryJoin {
                target_entity: "SectionEntity".into(),
                selections: vec![
                    QuerySelection {
                        field: "type".into(),
                        alias: None,
                    },
                    QuerySelection {
                        field: "num".into(),
                        alias: None,
                    },
                ],
                formulas: vec![],
                joins: vec![QueryJoin {
                    target_entity: "MeetingEntity".into(),
                    selections: vec![
                        QuerySelection {
                            field: "day".into(),
                            alias: None,
                        },
                        QuerySelection {
                            field: "start".into(),
                            alias: None,
                        },
                        QuerySelection {
                            field: "end".into(),
                            alias: None,
                        },
                    ],
                    formulas: vec![QueryFormula {
                        name: "duration".into(),
                        expression: Expression::BinaryOp {
                            left: Box::new(Expression::Field("end".into())),
                            op: BinaryOperator::Sub,
                            right: Box::new(Expression::Field("start".into())),
                        },
                    }],
                    joins: vec![],
                }],
            }],
            hierarchy: None,
        };

        schema.queries.insert("ActiveCourseQuery".into(), query);

        let runtime_params = std::collections::HashMap::new();
        let strategy = QueryEngine::plan(&schema, "ActiveCourseQuery", &runtime_params).expect("Failed to plan");

        assert_eq!(strategy.plans.len(), 1);
        let sql = if let QueryPlan::ExecuteSql { sql, .. } = &strategy.plans[0] {
            sql
        } else {
            panic!("Expected ExecuteSql");
        };
        println!("Generated SQL: {}", sql);
        assert!(sql.contains("SELECT course_entity.title"));
        assert!(sql.contains("SUM([duration]) AS total_duration"));
        assert!(sql.contains("FROM course_entity"));
        assert!(sql.contains("LEFT JOIN section_entity"));
        assert!(sql.contains("LEFT JOIN meeting_entity"));
    }

    #[test]
    fn test_lower_flat_query() {
        let mut schema = Schema::default();
        let query = QuerySchema {
            name: "BookQuery".into(),
            params: vec![],
            root_entity: "BookEntity".into(),
            query_type: QueryType::Flat,
            group_by: vec![],
            filters: vec![Expression::BinaryOp {
                left: Box::new(Expression::Field("published_at".into())),
                op: BinaryOperator::Sub, // Using sub as placeholder for comparison (no comparison op yet)
                // Wait, DSL needs comparison operators for filters.
                // The prompt example: `[published_at] < DATE('2000-01-01')`.
                // My `Expression` enum only supports basic arithmetic.
                // I need to add comparison operators to `BinaryOp` or `Expression`.
                // BUT, for now I will use existing operators just to verify WHERE clause generation.
                // Assuming parser parses `<` as something or I'll just use BinaryOp for now.
                // Wait, if parser doesn't support `<`, then user request `[published_at] < DATE` won't parse.
                // I need to update Expression/Parser to support `<`.
                right: Box::new(Expression::FunctionCall {
                    name: "DATE".into(),
                    args: vec![],
                }),
            }],
            selections: vec![
                QuerySelection {
                    field: "title".into(),
                    alias: None,
                },
                QuerySelection {
                    field: "price".into(),
                    alias: None,
                },
            ],
            formulas: vec![],
            joins: vec![QueryJoin {
                target_entity: "PeopleEntity".into(),
                selections: vec![QuerySelection {
                    field: "name".into(),
                    alias: Some("author".into()),
                }],
                formulas: vec![],
                joins: vec![],
            }],
            hierarchy: None,
        };
        schema.queries.insert("BookQuery".into(), query);

        let runtime_params = std::collections::HashMap::new();
        let strategy = QueryEngine::plan(&schema, "BookQuery", &runtime_params).expect("Failed to plan");
        let sql = if let QueryPlan::ExecuteSql { sql, .. } = &strategy.plans[0] {
            sql
        } else {
            panic!("Expected ExecuteSql");
        };
        println!("Flat SQL: {}", sql);

        assert!(sql.contains("SELECT book_entity.title, book_entity.price"));
        assert!(sql.contains("people_entity.name AS author"));
        assert!(sql.contains("FROM book_entity"));
        assert!(sql.contains("LEFT JOIN people_entity"));
        assert!(sql.contains("WHERE [published_at] - DATE()"));
    }

    #[test]
    fn test_parameterized_query() {
        let mut schema = Schema::default();
        let query = QuerySchema {
            name: "ParamQuery".into(),
            params: vec!["min_price".into()],
            root_entity: "Product".into(),
            query_type: QueryType::Flat,
            selections: vec![QuerySelection {
                field: "name".into(),
                alias: None,
            }],
            formulas: vec![],
            filters: vec![Expression::BinaryOp {
                left: Box::new(Expression::Field("price".into())),
                op: BinaryOperator::Gte,
                right: Box::new(Expression::FunctionCall {
                    name: "param".into(),
                    args: vec![Expression::StringLiteral("min_price".into())],
                }),
            }],
            joins: vec![],
            group_by: vec![],
            hierarchy: None,
        };
        schema.queries.insert("ParamQuery".into(), query);

        let mut runtime_params = std::collections::HashMap::new();
        runtime_params.insert("min_price".to_string(), Value::from(100));

        let strategy = QueryEngine::plan(&schema, "ParamQuery", &runtime_params).expect("Failed to plan");
        let (sql, params) = if let QueryPlan::ExecuteSql { sql, params } = &strategy.plans[0] {
            (sql, params)
        } else {
            panic!("Expected ExecuteSql");
        };

        println!("SQL: {}", sql);
        println!("Params: {:?}", params);

        assert!(sql.contains("WHERE [price] >="));
        // Check placeholder
        // Default db_type is Sqlite -> ?
        assert!(sql.contains("?"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], Value::from(100));
    }

    #[test]
    fn test_nested_field_expression() {
        let mut schema = Schema::default();
        let query = QuerySchema {
            name: "NestedQuery".into(),
            params: vec![],
            root_entity: "User".into(),
            query_type: QueryType::Flat,
            selections: vec![QuerySelection {
                field: "id".into(),
                alias: None,
            }],
            formulas: vec![],
            filters: vec![Expression::BinaryOp {
                left: Box::new(Expression::Field("user.profile.age".into())),
                op: BinaryOperator::Gte,
                right: Box::new(Expression::Literal(18.0)),
            }],
            joins: vec![],
            group_by: vec![],
            hierarchy: None,
        };
        schema.queries.insert("NestedQuery".into(), query);

        let runtime_params = std::collections::HashMap::new();
        let strategy = QueryEngine::plan(&schema, "NestedQuery", &runtime_params).expect("Failed to plan");
        let sql = if let QueryPlan::ExecuteSql { sql, .. } = &strategy.plans[0] {
            sql
        } else {
            panic!("Expected ExecuteSql");
        };

        println!("Nested SQL: {}", sql);

        // Verify that user.profile.age is converted to [user].[profile].[age]
        assert!(sql.contains("[user].[profile].[age]"));
    }

    #[test]
    fn test_reporting_functions() {
        let mut schema = Schema::default();
        let query = QuerySchema {
            name: "ReportQuery".into(),
            params: vec![],
            root_entity: "Account".into(),
            query_type: QueryType::Flat,
            selections: vec![QuerySelection {
                field: "id".into(),
                alias: None,
            }],
            formulas: vec![
                QueryFormula {
                    name: "conditional_balance".into(),
                    expression: Expression::FunctionCall {
                        name: "if".into(),
                        args: vec![
                            Expression::BoolLiteral(true),
                            Expression::Literal(100.0),
                            Expression::Literal(0.0),
                        ],
                    },
                },
                QueryFormula {
                    name: "running_balance".into(),
                    expression: Expression::FunctionCall {
                        name: "running_sum".into(),
                        args: vec![
                            Expression::Field("amount".into()),
                            Expression::Field("account_id".into()),
                            Expression::Field("date".into()),
                        ],
                    },
                },
            ],
            filters: vec![],
            joins: vec![],
            group_by: vec![],
            hierarchy: None,
        };
        schema.queries.insert("ReportQuery".into(), query);

        let runtime_params = std::collections::HashMap::new();
        let strategy = QueryEngine::plan(&schema, "ReportQuery", &runtime_params).expect("Failed to plan");
        let sql = if let QueryPlan::ExecuteSql { sql, .. } = &strategy.plans[0] {
            sql
        } else {
            panic!("Expected ExecuteSql");
        };

        println!("Report SQL: {}", sql);

        assert!(sql.contains("CASE WHEN TRUE THEN 100 ELSE 0 END AS conditional_balance"));
        assert!(sql.contains(
            "SUM([amount]) OVER (PARTITION BY [account_id] ORDER BY [date] ROWS UNBOUNDED PRECEDING) AS running_balance"
        ));
    }
}
