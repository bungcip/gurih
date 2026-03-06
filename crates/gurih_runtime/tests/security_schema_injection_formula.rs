use gurih_ir::{Expression, QueryFormula, QuerySchema, QueryType, Schema};
use gurih_runtime::query_engine::QueryEngine;
use std::collections::HashMap;

#[test]
fn test_formula_name_sqli() {
    let mut schema = Schema::default();
    let query = QuerySchema {
        name: "TestQuery".into(),
        params: vec![],
        root_entity: "User".into(),
        query_type: QueryType::Flat,
        selections: vec![],
        formulas: vec![QueryFormula {
            name: "admin; DROP TABLE users; --".into(),
            expression: Expression::Literal(1.0),
        }],
        filters: vec![],
        joins: vec![],
        group_by: vec![],
        hierarchy: None,
    };
    schema.queries.insert("TestQuery".into(), query);

    let runtime_params = HashMap::new();
    let result = QueryEngine::plan(&schema, "TestQuery", &runtime_params);

    assert!(result.is_err(), "Should have failed formula name validation");
}
