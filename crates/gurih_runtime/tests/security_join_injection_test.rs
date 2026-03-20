use gurih_ir::{
    EntitySchema, FieldSchema, FieldType, Ownership, QueryJoin, QuerySchema, QueryType, RelationshipSchema,
    RelationshipType, Schema, Symbol,
};
use std::collections::HashMap;
use gurih_runtime::query_engine::QueryEngine;

#[tokio::test]
async fn test_query_join_injection_check() {
    let mut schema = Schema::default();

    // 1. Define 'User' Entity
    let user_sym = Symbol::from("User");
    let user_entity = EntitySchema {
        name: user_sym,
        table_name: Symbol::from("users"),
        fields: vec![
            FieldSchema {
                name: Symbol::from("id"),
                field_type: FieldType::Pk,
                required: true,
                unique: true,
                default: None,
                references: None,
                serial_generator: None,
                storage: None,
                resize: None,
                filetype: None,
            },
        ],
        relationships: vec![RelationshipSchema {
            name: Symbol::from("malicious\"; DROP TABLE users; --"),
            target_entity: Symbol::from("Post"),
            rel_type: RelationshipType::BelongsTo,
            ownership: Ownership::Reference,
        }],
        options: HashMap::new(),
        seeds: None,
    };
    schema.entities.insert(user_sym, user_entity);

    // 2. Define 'Post' Entity
    let post_sym = Symbol::from("Post");
    let post_entity = EntitySchema {
        name: post_sym,
        table_name: Symbol::from("posts"),
        fields: vec![
            FieldSchema {
                name: Symbol::from("id"),
                field_type: FieldType::Pk,
                required: true,
                unique: true,
                default: None,
                references: None,
                serial_generator: None,
                storage: None,
                resize: None,
                filetype: None,
            },
        ],
        relationships: vec![],
        options: HashMap::new(),
        seeds: None,
    };
    schema.entities.insert(post_sym, post_entity);

    // 3. Define Query 'UserPosts' that joins User -> Post
    let query_sym = Symbol::from("UserPosts");
    let query = QuerySchema {
        name: query_sym,
        params: vec![],
        root_entity: user_sym,
        query_type: QueryType::Flat,
        selections: vec![],
        formulas: vec![],
        filters: vec![],
        joins: vec![QueryJoin {
            target_entity: post_sym,
            selections: vec![],
            formulas: vec![],
            joins: vec![],
        }],
        group_by: vec![],
        hierarchy: None,
    };
    schema.queries.insert(query_sym, query);

    let runtime_params = std::collections::HashMap::new();
    let result = QueryEngine::plan(&schema, "UserPosts", &runtime_params);
    assert!(result.is_err(), "Should fail identifier validation for rel.name in join");
    assert!(result.unwrap_err().contains("Invalid identifier"));
}
