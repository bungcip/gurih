use gurih_ir::{
    EntitySchema, FieldSchema, FieldType, QueryJoin, QuerySchema, QuerySelection, QueryType, Schema, Symbol,
};
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::store::MemoryDataStore;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_query_joined_permission_check() {
    let mut schema = Schema::default();

    // 1. Define 'User' Entity (Perm: read:User)
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
            FieldSchema {
                name: Symbol::from("name"),
                field_type: FieldType::String,
                required: true,
                unique: false,
                default: None,
                references: None,
                serial_generator: None,
                storage: None,
                resize: None,
                filetype: None,
            },
        ],
        relationships: vec![],
        options: HashMap::from([("read_permission".to_string(), "read:User".to_string())]),
        seeds: None,
    };
    schema.entities.insert(user_sym, user_entity);

    // 2. Define 'Post' Entity (Perm: read:Post)
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
            FieldSchema {
                name: Symbol::from("user_id"),
                field_type: FieldType::Uuid,
                required: true,
                unique: false,
                default: None,
                references: Some(user_sym),
                serial_generator: None,
                storage: None,
                resize: None,
                filetype: None,
            },
            FieldSchema {
                name: Symbol::from("title"),
                field_type: FieldType::String,
                required: true,
                unique: false,
                default: None,
                references: None,
                serial_generator: None,
                storage: None,
                resize: None,
                filetype: None,
            },
        ],
        relationships: vec![],
        options: HashMap::from([("read_permission".to_string(), "read:Post".to_string())]),
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
        selections: vec![QuerySelection {
            field: Symbol::from("name"),
            alias: None,
        }],
        formulas: vec![],
        filters: vec![],
        joins: vec![QueryJoin {
            target_entity: post_sym,
            selections: vec![QuerySelection {
                field: Symbol::from("title"),
                alias: None,
            }],
            formulas: vec![],
            joins: vec![],
        }],
        group_by: vec![],
        hierarchy: None,
    };
    schema.queries.insert(query_sym, query);

    let schema_arc = Arc::new(schema);
    let datastore = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(schema_arc, datastore.clone());

    // 4. Test Context: User has 'read:User' but NOT 'read:Post'
    let ctx = RuntimeContext {
        user_id: "test_user".to_string(),
        roles: vec![],
        permissions: vec!["read:User".to_string()], // Only permission for root entity
        token: None,
    };

    // 5. Execute Query
    let result = engine.list("UserPosts", None, None, None, &ctx).await;

    // 6. Assert Failure
    // If bug exists, this result will be OK (because only root entity checked).
    // If fixed, this result will be Err("Missing permission 'read:Post'").
    match result {
        Ok(_) => panic!("SECURITY FAILURE: Query succeeded despite missing permission for joined entity 'Post'"),
        Err(e) => {
            assert!(e.contains("Missing permission"), "Unexpected error: {}", e);
            assert!(
                e.contains("read:Post"),
                "Error should mention missing 'read:Post' permission, got: {}",
                e
            );
        }
    }
}
