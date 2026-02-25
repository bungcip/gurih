use gurih_ir::{DatabaseType, Expression, QueryFormula, QuerySchema, QuerySelection, QueryType, Schema};
use gurih_runtime::query_engine::{QueryEngine, QueryPlan};
use gurih_runtime::evaluator::evaluate;
use serde_json::{json, Value};

#[test]
fn test_days_between_sql_sqlite() {
    let mut schema = Schema::default();
    schema.database = Some(gurih_ir::DatabaseSchema {
        db_type: DatabaseType::Sqlite,
        url: "sqlite::memory:".to_string(),
    });

    let query = QuerySchema {
        name: "DaysQuery".into(),
        params: vec![],
        root_entity: "Test".into(),
        query_type: QueryType::Flat,
        selections: vec![],
        formulas: vec![QueryFormula {
            name: "days".into(),
            expression: Expression::FunctionCall {
                name: "days_between".into(),
                args: vec![
                    Expression::StringLiteral("2024-01-10".into()),
                    Expression::StringLiteral("2024-01-01".into()),
                ],
            },
        }],
        filters: vec![],
        joins: vec![],
        group_by: vec![],
        hierarchy: None,
    };
    schema.queries.insert("DaysQuery".into(), query);

    let runtime_params = std::collections::HashMap::new();
    let strategy = QueryEngine::plan(&schema, "DaysQuery", &runtime_params).expect("Failed to plan");
    let sql = if let QueryPlan::ExecuteSql { sql, .. } = &strategy.plans[0] {
        sql
    } else {
        panic!("Expected ExecuteSql");
    };

    println!("SQLite SQL: {}", sql);
    assert!(sql.contains("CAST(julianday(?) - julianday(?) AS INTEGER)"));
}

#[test]
fn test_days_between_sql_postgres() {
    let mut schema = Schema::default();
    schema.database = Some(gurih_ir::DatabaseSchema {
        db_type: DatabaseType::Postgres,
        url: "postgres://host".to_string(),
    });

    let query = QuerySchema {
        name: "DaysQuery".into(),
        params: vec![],
        root_entity: "Test".into(),
        query_type: QueryType::Flat,
        selections: vec![],
        formulas: vec![QueryFormula {
            name: "days".into(),
            expression: Expression::FunctionCall {
                name: "days_between".into(),
                args: vec![
                    Expression::StringLiteral("2024-01-10".into()),
                    Expression::StringLiteral("2024-01-01".into()),
                ],
            },
        }],
        filters: vec![],
        joins: vec![],
        group_by: vec![],
        hierarchy: None,
    };
    schema.queries.insert("DaysQuery".into(), query);

    let runtime_params = std::collections::HashMap::new();
    let strategy = QueryEngine::plan(&schema, "DaysQuery", &runtime_params).expect("Failed to plan");
    let sql = if let QueryPlan::ExecuteSql { sql, .. } = &strategy.plans[0] {
        sql
    } else {
        panic!("Expected ExecuteSql");
    };

    println!("Postgres SQL: {}", sql);
    // Note: Parameter placeholders $1, $2
    assert!(sql.contains("($1::DATE - $2::DATE)"));
}

#[tokio::test]
async fn test_days_between_evaluator() {
    let expr = Expression::FunctionCall {
        name: "days_between".into(),
        args: vec![
            Expression::StringLiteral("2024-01-10".into()),
            Expression::StringLiteral("2024-01-01".into()),
        ],
    };
    let ctx = json!({});
    let res = evaluate(&expr, &ctx, None, None).await.unwrap();
    assert_eq!(res, json!(9));

    // Negative check
    let expr_neg = Expression::FunctionCall {
        name: "days_between".into(),
        args: vec![
            Expression::StringLiteral("2024-01-01".into()),
            Expression::StringLiteral("2024-01-10".into()),
        ],
    };
    let res_neg = evaluate(&expr_neg, &ctx, None, None).await.unwrap();
    assert_eq!(res_neg, json!(-9));
}
