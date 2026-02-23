use gurih_ir::{EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::MemoryDataStore;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_data_engine_list_respects_filters() {
    // 1. Setup Schema
    let mut schema = Schema::default();
    let entity_name = "User";

    let mut user_entity = EntitySchema {
        name: Symbol::from(entity_name),
        table_name: Symbol::from("users"),
        fields: vec![
            FieldSchema {
                name: Symbol::from("username"),
                field_type: FieldType::String,
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
                name: Symbol::from("role"),
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
        options: HashMap::new(),
        seeds: None,
    };
    // Allow read permissions for everyone for this test
    user_entity.options.insert("read_permission".to_string(), "public".to_string());
    user_entity.options.insert("create_permission".to_string(), "public".to_string());

    schema.entities.insert(Symbol::from(entity_name), user_entity);

    let datastore = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(Arc::new(schema), datastore);

    // Context with permission
    let ctx = RuntimeContext {
        user_id: "admin".to_string(),
        roles: vec!["admin".to_string()],
        permissions: vec!["public".to_string()],
        token: None,
    };

    // 2. Insert Data
    engine.create(entity_name, json!({
        "username": "alice",
        "role": "admin"
    }), &ctx).await.unwrap();

    engine.create(entity_name, json!({
        "username": "bob",
        "role": "user"
    }), &ctx).await.unwrap();

    // 3. List with Filter
    let mut filters = HashMap::new();
    filters.insert("role".to_string(), "admin".to_string());

    let results = engine.list(entity_name, None, None, Some(filters), &ctx).await.unwrap();

    // 4. Assert
    // Filters should be respected
    assert_eq!(results.len(), 1, "Filters should be respected");

    let usernames: Vec<String> = results.iter()
        .map(|r| r.get("username").and_then(|v| v.as_str()).unwrap().to_string())
        .collect();
    assert!(usernames.contains(&"alice".to_string()));
    assert!(!usernames.contains(&"bob".to_string()));
}
