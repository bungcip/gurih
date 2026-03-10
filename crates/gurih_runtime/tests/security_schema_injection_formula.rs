use gurih_ir::{Expression, QueryFormula, QuerySchema, QueryType, Schema};
use gurih_runtime::query_engine::QueryEngine;
use std::collections::HashMap;

#[test]
fn test_selection_field_sqli() {
    let mut schema = Schema::default();
    let query = QuerySchema {
        name: "TestQuery".into(),
        params: vec![],
        root_entity: "User".into(),
        query_type: QueryType::Flat,
        selections: vec![gurih_ir::QuerySelection {
            field: "admin; DROP TABLE users; --".into(),
            alias: None,
        }],
        formulas: vec![],
        filters: vec![],
        joins: vec![],
        group_by: vec![],
        hierarchy: None,
    };
    schema.queries.insert("TestQuery".into(), query);

    let runtime_params = HashMap::new();
    let result = QueryEngine::plan(&schema, "TestQuery", &runtime_params);

    assert!(result.is_err(), "Should have failed selection field validation");
}

#[test]
fn test_hierarchy_parent_field_sqli() {
    let mut schema = Schema::default();
    let query = QuerySchema {
        name: "TestQuery".into(),
        params: vec![],
        root_entity: "User".into(),
        query_type: QueryType::Hierarchy,
        selections: vec![],
        formulas: vec![],
        filters: vec![],
        joins: vec![],
        group_by: vec![],
        hierarchy: Some(gurih_ir::HierarchySchema {
            parent_field: "parent_id\"; DROP TABLE users; --".into(),
            rollup_fields: vec![],
        }),
    };
    schema.queries.insert("TestQuery".into(), query);

    let runtime_params = HashMap::new();
    let result = QueryEngine::plan(&schema, "TestQuery", &runtime_params);

    assert!(result.is_err(), "Should have failed hierarchy parent_field validation");
}

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
