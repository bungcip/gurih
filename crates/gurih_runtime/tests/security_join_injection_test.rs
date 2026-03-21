use gurih_ir::{Ownership, QueryJoin, QuerySchema, QuerySelection, QueryType, RelationshipSchema, RelationshipType, Schema};
use gurih_runtime::query_engine::QueryEngine;
use std::collections::HashMap;

#[test]
fn test_security_join_injection_prevented() {
    let mut schema = Schema::default();

    // Create a malicious schema where the relationship name contains SQL injection
    let parent_entity = gurih_ir::EntitySchema {
        name: "ParentEntity".into(),
        table_name: "parent_table".into(),
        fields: vec![],
        relationships: vec![RelationshipSchema {
            name: "malicious_rel\"; DROP TABLE users; --".into(), // Malicious name
            target_entity: "TargetEntity".into(),
            rel_type: RelationshipType::BelongsTo,
            ownership: Ownership::Reference,
        }],
        options: HashMap::new(),
        seeds: None,
    };

    schema.entities.insert("ParentEntity".into(), parent_entity);

    let query = QuerySchema {
        name: "TestQuery".into(),
        params: vec![],
        root_entity: "ParentEntity".into(),
        query_type: QueryType::Flat,
        filters: vec![],
        group_by: vec![],
        selections: vec![],
        formulas: vec![],
        joins: vec![QueryJoin {
            target_entity: "TargetEntity".into(),
            selections: vec![QuerySelection {
                field: "id".into(),
                alias: None,
            }],
            formulas: vec![],
            joins: vec![],
        }],
        hierarchy: None,
    };

    schema.queries.insert("TestQuery".into(), query);

    let runtime_params = HashMap::new();
    let result = QueryEngine::plan(&schema, "TestQuery", &runtime_params);

    // Assert that the injection was caught by validate_identifier and returned an error
    assert!(
        result.is_err(),
        "Expected query engine to reject malicious relationship name"
    );
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Invalid identifier"),
        "Expected validation error, got: {}",
        err
    );
}
