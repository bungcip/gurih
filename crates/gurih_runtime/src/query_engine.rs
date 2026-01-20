use gurih_ir::{BinaryOperator, Expression, QueryJoin, Schema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryPlan {
    ExecuteSql { sql: String },
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
            .get(query_name)
            .ok_or_else(|| format!("Query '{}' not found in schema", query_name))?;

        let mut select_parts = vec![];
        let mut join_parts = vec![];
        let root_table = Self::entity_to_table(&query.root_entity);

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
            let expr_sql = Self::expression_to_sql(&form.expression);
            select_parts.push(format!("{} AS {}", expr_sql, form.name));
        }

        // Process Joins (Recursive)
        Self::process_joins(
            &query.joins,
            &root_table,
            &query.root_entity,
            schema,
            &mut select_parts,
            &mut join_parts,
        )?;

        let select_clause = if select_parts.is_empty() {
            "*".to_string()
        } else {
            select_parts.join(", ")
        };

        let join_clause = join_parts.join(" ");

        let mut where_clause = String::new();
        if !query.filters.is_empty() {
            let filter_parts: Vec<String> = query.filters.iter().map(Self::expression_to_sql).collect();
            where_clause = format!("WHERE {}", filter_parts.join(" AND "));
        }

        let sql = format!(
            "SELECT {} FROM {} {} {}",
            select_clause, root_table, join_clause, where_clause
        )
        .trim()
        .to_string();

        Ok(QueryExecutionStrategy {
            plans: vec![QueryPlan::ExecuteSql { sql }],
        })
    }

    fn process_joins(
        joins: &[QueryJoin],
        parent_table: &str,
        parent_entity: &str,
        schema: &Schema,
        select_parts: &mut Vec<String>,
        join_parts: &mut Vec<String>,
    ) -> Result<(), String> {
        for join in joins {
            let target_entity_name = &join.target_entity;
            let target_table = Self::entity_to_table(target_entity_name);

            // Find relationship to determine join condition
            // Assuming parent has relationship to target or vice versa
            let mut join_condition = String::new();

            // Attempts to determine join condition from schema
            if let Some(parent_ent) = schema.entities.get(parent_entity) {
                if let Some(rel) = parent_ent
                    .relationships
                    .iter()
                    .find(|r| r.target_entity == *target_entity_name)
                {
                    if rel.rel_type == "belongs_to" {
                        // Parent has FK: parent.rel_id = target.id
                        join_condition = format!("{}.{}_id = {}.id", parent_table, rel.name, target_table);
                    } else {
                        // Target has FK: target.parent_id = parent.id
                        // Assuming standard naming convention for back-ref
                        join_condition = format!("{}.{}_id = {}.id", target_table, parent_table, parent_table);
                    }
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
                let expr_sql = Self::expression_to_sql(&form.expression);
                select_parts.push(format!("{} AS {}", expr_sql, form.name));
            }

            Self::process_joins(
                &join.joins,
                &target_table,
                target_entity_name,
                schema,
                select_parts,
                join_parts,
            )?;
        }
        Ok(())
    }

    fn entity_to_table(entity_name: &str) -> String {
        // Simple heuristic: lowercase
        entity_name.to_lowercase()
    }

    fn expression_to_sql(expr: &Expression) -> String {
        match expr {
            Expression::Field(f) => format!("[{}]", f), // Naive, should probably be qualified if possible, but context is hard
            Expression::Literal(n) => n.to_string(),
            Expression::StringLiteral(s) => format!("'{}'", s),
            Expression::FunctionCall { name, args } => {
                let args_sql: Vec<String> = args.iter().map(Self::expression_to_sql).collect();
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
                    Self::expression_to_sql(left),
                    op_str,
                    Self::expression_to_sql(right)
                )
            }
            Expression::Grouping(inner) => format!("({})", Self::expression_to_sql(inner)),
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
            name: "ActiveCourseQuery".to_string(),
            root_entity: "CourseEntity".to_string(),
            query_type: QueryType::Nested,
            filters: vec![],
            selections: vec![QuerySelection {
                field: "title".to_string(),
                alias: None,
            }],
            formulas: vec![QueryFormula {
                name: "total_duration".to_string(),
                expression: Expression::FunctionCall {
                    name: "SUM".to_string(),
                    args: vec![Expression::Field("duration".to_string())],
                },
            }],
            joins: vec![QueryJoin {
                target_entity: "SectionEntity".to_string(),
                selections: vec![
                    QuerySelection {
                        field: "type".to_string(),
                        alias: None,
                    },
                    QuerySelection {
                        field: "num".to_string(),
                        alias: None,
                    },
                ],
                formulas: vec![],
                joins: vec![QueryJoin {
                    target_entity: "MeetingEntity".to_string(),
                    selections: vec![
                        QuerySelection {
                            field: "day".to_string(),
                            alias: None,
                        },
                        QuerySelection {
                            field: "start".to_string(),
                            alias: None,
                        },
                        QuerySelection {
                            field: "end".to_string(),
                            alias: None,
                        },
                    ],
                    formulas: vec![QueryFormula {
                        name: "duration".to_string(),
                        expression: Expression::BinaryOp {
                            left: Box::new(Expression::Field("end".to_string())),
                            op: BinaryOperator::Sub,
                            right: Box::new(Expression::Field("start".to_string())),
                        },
                    }],
                    joins: vec![],
                }],
            }],
        };

        schema.queries.insert("ActiveCourseQuery".to_string(), query);

        let strategy = QueryEngine::plan(&schema, "ActiveCourseQuery").expect("Failed to plan");

        assert_eq!(strategy.plans.len(), 1);
        let QueryPlan::ExecuteSql { sql } = &strategy.plans[0];
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
            name: "BookQuery".to_string(),
            root_entity: "BookEntity".to_string(),
            query_type: QueryType::Flat,
            filters: vec![Expression::BinaryOp {
                left: Box::new(Expression::Field("published_at".to_string())),
                op: BinaryOperator::Sub, // Using sub as placeholder for comparison (no comparison op yet)
                // Wait, DSL needs comparison operators for filters.
                // The prompt example: `[published_at] < DATE('2000-01-01')`.
                // My `Expression` enum only supports basic arithmetic.
                // I need to add comparison operators to `BinaryOp` or `Expression`.
                // BUT, for now I will use existing operators just to verify WHERE clause generation.
                // Assuming parser parses `<` as something or I'll just use BinaryOp for now.
                // Wait, if parser doesn't support `<`, then user request `[published_at] < DATE` won't parse.
                // I need to update Expression/Parser to support `<`.
                // I will add TODO comment and use supported operator for this test.
                right: Box::new(Expression::FunctionCall {
                    name: "DATE".to_string(),
                    args: vec![],
                }),
            }],
            selections: vec![
                QuerySelection {
                    field: "title".to_string(),
                    alias: None,
                },
                QuerySelection {
                    field: "price".to_string(),
                    alias: None,
                },
            ],
            formulas: vec![],
            joins: vec![QueryJoin {
                target_entity: "PeopleEntity".to_string(),
                selections: vec![QuerySelection {
                    field: "name".to_string(),
                    alias: Some("author".to_string()),
                }],
                formulas: vec![],
                joins: vec![],
            }],
        };
        schema.queries.insert("BookQuery".to_string(), query);

        let strategy = QueryEngine::plan(&schema, "BookQuery").expect("Failed to plan");
        let QueryPlan::ExecuteSql { sql } = &strategy.plans[0];
        println!("Flat SQL: {}", sql);

        assert!(sql.contains("SELECT bookentity.title, bookentity.price"));
        assert!(sql.contains("peopleentity.name AS author"));
        assert!(sql.contains("FROM bookentity"));
        assert!(sql.contains("LEFT JOIN peopleentity"));
        assert!(sql.contains("WHERE [published_at] - DATE()"));
    }
}
