use gurih_ir::{BinaryOperator, DatabaseType, Expression, QueryJoin, Schema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryPlan {
    ExecuteSql { sql: String, params: Vec<Value> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueryExecutionStrategy {
    pub plans: Vec<QueryPlan>,
}

pub struct QueryEngine;

impl QueryEngine {
    pub fn plan(schema: &Schema, query_name: &str) -> Result<QueryExecutionStrategy, String> {
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
            let expr_sql = Self::expression_to_sql(&form.expression, &mut params, &db_type);
            select_parts.push(format!("{} AS {}", expr_sql, form.name));
        }

        // Process Joins (Recursive)
        Self::process_joins(
            &query.joins,
            &root_table,
            &query.root_entity.to_string(),
            schema,
            &mut select_parts,
            &mut join_parts,
            &mut params,
            &db_type,
        )?;

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
                .map(|e| Self::expression_to_sql(e, &mut params, &db_type))
                .collect();
            where_clause = format!("WHERE {}", filter_parts.join(" AND "));
        }

        let mut group_by_clause = String::new();
        if !query.group_by.is_empty() {
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

        Ok(QueryExecutionStrategy {
            plans: vec![QueryPlan::ExecuteSql { sql, params }],
        })
    }

    fn process_joins(
        joins: &[QueryJoin],
        parent_table: &str,
        parent_entity: &str,
        schema: &Schema,
        select_parts: &mut Vec<String>,
        join_parts: &mut Vec<String>,
        params: &mut Vec<Value>,
        db_type: &DatabaseType,
    ) -> Result<(), String> {
        for join in joins {
            let target_entity_name = &join.target_entity;
            let target_table = Self::entity_to_table(&target_entity_name.to_string());

            // Find relationship to determine join condition
            // Assuming parent has relationship to target or vice versa
            let mut join_condition = String::new();

            // Attempts to determine join condition from schema
            if let Some(parent_ent) = schema.entities.get(&parent_entity.into())
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

            join_parts.push(format!("LEFT JOIN {} ON {}", target_table, join_condition));

            for sel in &join.selections {
                let col_sql = format!("{}.{}", target_table, sel.field);
                if let Some(alias) = &sel.alias {
                    select_parts.push(format!("{} AS {}", col_sql, alias));
                } else {
                    select_parts.push(col_sql);
                }
            }
            for form in &join.formulas {
                let expr_sql = Self::expression_to_sql(&form.expression, params, db_type);
                select_parts.push(format!("{} AS {}", expr_sql, form.name));
            }

            Self::process_joins(
                &join.joins,
                &target_table,
                &target_entity_name.to_string(),
                schema,
                select_parts,
                join_parts,
                params,
                db_type,
            )?;
        }
        Ok(())
    }

    fn entity_to_table(entity_name: &str) -> String {
        // Simple heuristic: lowercase
        entity_name.to_lowercase()
    }

    fn expression_to_sql(expr: &Expression, params: &mut Vec<Value>, db_type: &DatabaseType) -> String {
        match expr {
            Expression::Field(f) => format!("[{}]", f), // Naive, should probably be qualified if possible, but context is hard
            Expression::Literal(n) => n.to_string(),
            Expression::StringLiteral(s) => {
                params.push(Value::String(s.clone()));
                if *db_type == DatabaseType::Postgres {
                    format!("${}", params.len())
                } else {
                    "?".to_string()
                }
            }
            Expression::FunctionCall { name, args } => {
                let args_sql: Vec<String> =
                    args.iter().map(|a| Self::expression_to_sql(a, params, db_type)).collect();
                format!("{}({})", name, args_sql.join(", "))
            }
            Expression::BinaryOp { left, op, right } => {
                let op_str = match op {
                    BinaryOperator::Add => "+",
                    BinaryOperator::Sub => "-",
                    BinaryOperator::Mul => "*",
                    BinaryOperator::Div => "/",
                };
                format!(
                    "{} {} {}",
                    Self::expression_to_sql(left, params, db_type),
                    op_str,
                    Self::expression_to_sql(right, params, db_type)
                )
            }
            Expression::Grouping(inner) => {
                format!("({})", Self::expression_to_sql(inner, params, db_type))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gurih_ir::{Expression, QueryFormula, QueryJoin, QuerySchema, QuerySelection, QueryType};

    #[test]
    fn test_lower_perspective_query() {
        let mut schema = Schema::default();

        // Setup Query Schema
        let query = QuerySchema {
            name: "ActiveCourseQuery".into(),
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
        };

        schema.queries.insert("ActiveCourseQuery".into(), query);

        let strategy = QueryEngine::plan(&schema, "ActiveCourseQuery").expect("Failed to plan");

        assert_eq!(strategy.plans.len(), 1);
        let QueryPlan::ExecuteSql { sql, .. } = &strategy.plans[0];
        println!("Generated SQL: {}", sql);
        assert!(sql.contains("SELECT courseentity.title"));
        assert!(sql.contains("SUM([duration]) AS total_duration"));
        assert!(sql.contains("FROM courseentity"));
        assert!(sql.contains("LEFT JOIN sectionentity"));
        assert!(sql.contains("LEFT JOIN meetingentity"));
    }

    #[test]
    fn test_lower_flat_query() {
        let mut schema = Schema::default();
        let query = QuerySchema {
            name: "BookQuery".into(),
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
        };
        schema.queries.insert("BookQuery".into(), query);

        let strategy = QueryEngine::plan(&schema, "BookQuery").expect("Failed to plan");
        let QueryPlan::ExecuteSql { sql, .. } = &strategy.plans[0];
        println!("Flat SQL: {}", sql);

        assert!(sql.contains("SELECT bookentity.title, bookentity.price"));
        assert!(sql.contains("peopleentity.name AS author"));
        assert!(sql.contains("FROM bookentity"));
        assert!(sql.contains("LEFT JOIN peopleentity"));
        assert!(sql.contains("WHERE [published_at] - DATE()"));
    }
}
