use gurih_ir::{QuerySchema, QueryType, Schema, QuerySelection, QueryFormula, Expression};
use gurih_runtime::query_engine::QueryEngine;
use std::collections::HashMap;

#[test]
fn test_sql_injection_in_formula_name() {
    let mut schema = Schema::default();

    let malicious_formula_name = "malicious_formula\"; DROP TABLE users; --";

    let query = QuerySchema {
        name: "MaliciousFormulaQuery".into(),
        params: vec![],
        root_entity: "User".into(), // Valid entity
        query_type: QueryType::Flat,
        filters: vec![],
        group_by: vec![],
        selections: vec![],
        formulas: vec![QueryFormula {
            name: malicious_formula_name.into(),
            expression: Expression::StringLiteral("safe_string".to_string()), // Safe expression
        }],
        joins: vec![],
        hierarchy: None,
    };

    schema.queries.insert("MaliciousFormulaQuery".into(), query);

    let runtime_params = HashMap::new();

    let result = QueryEngine::plan(&schema, "MaliciousFormulaQuery", &runtime_params);

    assert!(result.is_err(), "Query planning should fail for malicious formula name");
    let err = result.err().unwrap();
    assert!(
        err.contains("Invalid identifier"),
        "Error should mention invalid identifier, got: {}",
        err
    );
}

#[test]
fn test_sql_injection_in_selection_alias() {
    let mut schema = Schema::default();

    let malicious_alias = "malicious_alias\"; DROP TABLE users; --";

    let query = QuerySchema {
        name: "MaliciousAliasQuery".into(),
        params: vec![],
        root_entity: "User".into(), // Valid entity
        query_type: QueryType::Flat,
        filters: vec![],
        group_by: vec![],
        selections: vec![QuerySelection {
            field: "id".into(),
            alias: Some(malicious_alias.into()),
        }],
        formulas: vec![],
        joins: vec![],
        hierarchy: None,
    };

    schema.queries.insert("MaliciousAliasQuery".into(), query);

    let runtime_params = HashMap::new();

    let result = QueryEngine::plan(&schema, "MaliciousAliasQuery", &runtime_params);

    assert!(result.is_err(), "Query planning should fail for malicious selection alias");
    let err = result.err().unwrap();
    assert!(
        err.contains("Invalid identifier"),
        "Error should mention invalid identifier, got: {}",
        err
    );
}
