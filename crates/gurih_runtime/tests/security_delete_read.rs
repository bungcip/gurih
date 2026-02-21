use gurih_ir::{EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::{DataStore, MemoryDataStore};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_delete_requires_read_permission() {
    let mut schema = Schema::default();
    let entity_name = "TestEntity";
    let entity_sym = Symbol::from(entity_name);

    let fields = vec![
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
    ];

    let entity = EntitySchema {
        name: entity_sym.clone(),
        table_name: Symbol::from("test_entities"),
        fields,
        relationships: vec![],
        options: HashMap::new(),
        seeds: None,
    };

    schema.entities.insert(entity_sym.clone(), entity);

    let schema_arc = Arc::new(schema);
    let datastore = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(schema_arc, datastore.clone());

    // Insert a record
    datastore
        .insert(
            "test_entities",
            json!({
                "id": "1",
                "name": "To be deleted"
            }),
        )
        .await
        .unwrap();

    // Context with ONLY delete permission
    let ctx = RuntimeContext {
        user_id: "deleter".to_string(),
        roles: vec![],
        permissions: vec![format!("delete:{}", entity_name)], // Only delete permission
        token: None,
    };

    // Attempt delete
    let result = engine.delete(entity_name, "1", &ctx).await;

    // This should FAIL if we enforce read permission
    assert!(result.is_err(), "Delete should fail if user lacks read permission");
    let err = result.err().unwrap();
    assert!(
        err.contains("Missing permission 'read:TestEntity'"),
        "Error message should mention missing read permission. Got: {}",
        err
    );
}
