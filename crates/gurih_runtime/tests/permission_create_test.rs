use gurih_ir::{EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::MemoryDataStore;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_create_permission_missing() {
    // 1. Setup Schema
    let mut entities = HashMap::new();
    let fields = vec![
        FieldSchema {
            name: Symbol::from("id"),
            field_type: FieldType::String,
            required: false,
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
    ];

    entities.insert(
        Symbol::from("TestEntity"),
        EntitySchema {
            name: Symbol::from("TestEntity"),
            table_name: Symbol::from("test_entity"),
            fields,
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    let schema = Schema {
        entities,
        ..Default::default()
    };
    let schema_arc = Arc::new(schema);

    // 2. Setup Engine
    let datastore = Arc::new(MemoryDataStore::new());
    let data_engine = DataEngine::new(schema_arc.clone(), datastore.clone());

    // 3. Create Context with NO permissions
    let ctx = RuntimeContext {
        user_id: "user1".to_string(),
        roles: vec!["user".to_string()],
        permissions: vec![], // No permissions
        token: None,
    };

    // 4. Attempt Create -> Should Fail
    let result = data_engine
        .create(
            "TestEntity",
            json!({
                "name": "Test Item"
            }),
            &ctx,
        )
        .await;

    assert!(result.is_err(), "Create should fail when permission is missing");
    if let Err(err) = result {
        assert!(
            err.contains("Missing permission"),
            "Error message should mention missing permission: {}",
            err
        );
    }
}

#[tokio::test]
async fn test_create_permission_granted() {
    // 1. Setup Schema
    let mut entities = HashMap::new();
    let fields = vec![
        FieldSchema {
            name: Symbol::from("id"),
            field_type: FieldType::String,
            required: false,
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
    ];

    entities.insert(
        Symbol::from("TestEntity"),
        EntitySchema {
            name: Symbol::from("TestEntity"),
            table_name: Symbol::from("test_entity"),
            fields,
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    let schema = Schema {
        entities,
        ..Default::default()
    };
    let schema_arc = Arc::new(schema);

    // 2. Setup Engine
    let datastore = Arc::new(MemoryDataStore::new());
    let data_engine = DataEngine::new(schema_arc.clone(), datastore.clone());

    // 3. Create Context WITH permission
    let ctx = RuntimeContext {
        user_id: "user2".to_string(),
        roles: vec!["user".to_string()],
        permissions: vec!["create:TestEntity".to_string()], // Has permission
        token: None,
    };

    // 4. Attempt Create -> Should Succeed
    let result = data_engine
        .create(
            "TestEntity",
            json!({
                "name": "Test Item"
            }),
            &ctx,
        )
        .await;

    assert!(
        result.is_ok(),
        "Create should succeed when permission is granted. Error: {:?}",
        result.err()
    );
}
