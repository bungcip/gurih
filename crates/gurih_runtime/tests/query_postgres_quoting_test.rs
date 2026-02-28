use gurih_ir::{
    BinaryOperator, DatabaseSchema, DatabaseType, Expression, QuerySchema, QuerySelection, QueryType, Schema,
};
use gurih_runtime::query_engine::{QueryEngine, QueryPlan};
use std::collections::HashMap;

#[test]
fn test_postgres_sql_generation() {
    let mut schema = Schema {
        database: Some(DatabaseSchema {
            db_type: DatabaseType::Postgres,
            url: "postgres://localhost:5432/test".into(),
        }),
        ..Default::default()
    };

    let query = QuerySchema {
        name: "TestQuery".into(),
        params: vec![],
        root_entity: "User".into(),
        query_type: QueryType::Flat,
        selections: vec![QuerySelection {
            field: "id".into(),
            alias: None,
        }],
        formulas: vec![],
        filters: vec![Expression::BinaryOp {
            left: Box::new(Expression::Field("user.name".into())),
            op: BinaryOperator::Eq,
            right: Box::new(Expression::StringLiteral("admin".into())),
        }],
        joins: vec![],
        group_by: vec![],
        hierarchy: None,
    };
    schema.queries.insert("TestQuery".into(), query);

    let runtime_params = HashMap::new();
    let strategy = QueryEngine::plan(&schema, "TestQuery", &runtime_params).expect("Failed to plan");

    if let QueryPlan::ExecuteSql { sql, .. } = &strategy.plans[0] {
        println!("Generated SQL for Postgres: {}", sql);

        // Check filter clause
        assert!(
            sql.contains("\"user\".\"name\""),
            "Should use double quotes for Postgres identifier quoting"
        );
        assert!(!sql.contains("[user].[name]"), "Should not use square brackets");

        // Check SELECT and FROM
        assert!(sql.contains("\"user\".\"id\""), "Should quote selected columns");
        assert!(sql.contains("FROM \"user\""), "Should quote table name");
    } else {
        panic!("Expected ExecuteSql");
    }
}
