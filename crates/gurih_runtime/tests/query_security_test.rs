use gurih_ir::{DatabaseSchema, DatabaseType, Expression, QuerySchema, QuerySelection, QueryType, Schema};
use gurih_runtime::query_engine::{QueryEngine, QueryPlan};

#[test]
fn test_sql_injection_reproduction() {
    let mut schema = Schema {
        database: Some(DatabaseSchema {
            db_type: DatabaseType::Sqlite,
            url: "sqlite::memory:".to_string(),
        }),
        ..Default::default()
    };

    // Malicious payload that changes logic: WHERE name + '' OR '1'='1'
    let malicious_input = "' OR '1'='1";

    // Use BinaryOperator to construct a WHERE clause
    // Since BinaryOperator only has Add/Sub/Mul/Div, we construct: [name] + 'malicious'
    // This is silly but sufficient to prove string injection.
    // If [name] + '' OR '1'='1' is generated, injection is proven.
    let query = QuerySchema {
        name: "InjectionQuery".into(),
        root_entity: "User".into(),
        query_type: QueryType::Flat,
        filters: vec![Expression::BinaryOp {
            left: Box::new(Expression::Field("name".into())),
            op: gurih_ir::BinaryOperator::Add,
            right: Box::new(Expression::StringLiteral(malicious_input.to_string())),
        }],
        group_by: vec![],
        selections: vec![QuerySelection {
            field: "username".into(),
            alias: None,
        }],
        formulas: vec![],
        joins: vec![],
    };

    schema.queries.insert("InjectionQuery".into(), query);

    let strategy = QueryEngine::plan(&schema, "InjectionQuery").expect("Failed to plan");
    let QueryPlan::ExecuteSql { sql, .. } = &strategy.plans[0];

    println!("Generated SQL: {}", sql);

    // Check if the single quote is escaped or parameterized.
    // Vulnerable: ... + '' OR '1'='1'
    // Safe (Parameterized): ... + ?  (and params contain "' OR '1'='1")
    // Safe (Escaped): ... + ''' OR ''1''=''1'''

    // We expect the vulnerability to be GONE and placeholders used
    assert!(
        !sql.contains("' OR '1'='1'"),
        "SQL Injection vulnerability fix verification: payload should NOT be in SQL string"
    );
    assert!(sql.contains("?"), "SQL should use placeholders for string literals");

    let QueryPlan::ExecuteSql { sql: _, params } = &strategy.plans[0];
    assert_eq!(params.len(), 1);
    assert_eq!(params[0], serde_json::Value::String(malicious_input.to_string()));
}
