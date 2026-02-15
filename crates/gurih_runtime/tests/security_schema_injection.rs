use gurih_ir::{QuerySchema, QueryType, Schema};
use gurih_runtime::query_engine::QueryEngine;
use std::collections::HashMap;

#[test]
fn test_sql_injection_in_schema_entity_name() {
    let mut schema = Schema::default();

    // Malicious entity name (lowercase to bypass camel->snake conversion artifacts)
    let malicious_entity = "user; drop table users; --";

    // Setup Query Schema with malicious root entity
    let query = QuerySchema {
        name: "MaliciousQuery".into(),
        params: vec![],
        root_entity: malicious_entity.into(),
        query_type: QueryType::Flat,
        filters: vec![],
        group_by: vec![],
        selections: vec![],
        formulas: vec![],
        joins: vec![],
        hierarchy: None,
    };

    schema.queries.insert("MaliciousQuery".into(), query);

    let runtime_params = HashMap::new();

    // This should fail if validation is in place
    let result = QueryEngine::plan(&schema, "MaliciousQuery", &runtime_params);

    assert!(result.is_err(), "Query planning should fail for malicious entity name");
    let err = result.err().unwrap();
    // Assuming validate_identifier returns error starting with "Invalid identifier"
    assert!(err.contains("Invalid identifier"), "Error should mention invalid identifier, got: {}", err);
}

#[test]
fn test_sql_injection_in_group_by() {
    let mut schema = Schema::default();

    let malicious_field = "id] --";

    let query = QuerySchema {
        name: "MaliciousGroupBy".into(),
        params: vec![],
        root_entity: "User".into(), // Valid entity
        query_type: QueryType::Flat,
        filters: vec![],
        group_by: vec![malicious_field.into()], // Malicious field
        selections: vec![],
        formulas: vec![],
        joins: vec![],
        hierarchy: None,
    };

    schema.queries.insert("MaliciousGroupBy".into(), query);
    let runtime_params = HashMap::new();

    let result = QueryEngine::plan(&schema, "MaliciousGroupBy", &runtime_params);

    assert!(result.is_err(), "Query planning should fail for malicious group_by field");
    let err = result.err().unwrap();
    assert!(err.contains("Invalid identifier"), "Error should mention invalid identifier, got: {}", err);
}
