use gurih_ir::{ActionLogic, ActionStep, EntitySchema, FieldSchema, FieldType, Schema};
use gurih_runtime::action::ActionEngine;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::storage::MemoryStorage;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_action_execution_delete() {
    // 1. Setup minimal Schema with "Item" entity
    let mut entities = HashMap::new();

    // Entity: Item
    let fields = vec![
        FieldSchema {
            name: "id".to_string(),
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
            name: "name".to_string(),
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
        "Item".to_string(),
        EntitySchema {
            name: "Item".to_string(), // Missing in previous test
            fields,
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // 2. Setup Action Definition
    let mut actions = HashMap::new();
    let mut args = HashMap::new();
    args.insert("id".to_string(), "param(\"id\")".to_string());

    actions.insert(
        "DeleteItem".to_string(),
        ActionLogic {
            name: "DeleteItem".to_string(),
            params: vec!["id".to_string()],
            steps: vec![ActionStep {
                step_type: "entity:delete".to_string(),
                target: "Item".to_string(),
                args,
            }],
        },
    );

    let schema = Schema {
        entities,
        actions: actions.clone(),
        ..Default::default()
    };
    let schema_arc = Arc::new(schema);

    // 3. Setup Engines
    let storage = Arc::new(MemoryStorage::new());
    let data_engine = DataEngine::new(schema_arc.clone(), storage.clone());
    let action_engine = ActionEngine::new(actions);

    // 4. Pre-populate Data
    let ctx = RuntimeContext::system();
    // data_engine.create expects Value.
    // Usually it returns ID string.
    let id = data_engine
        .create("Item", json!({"id": "item-1", "name": "Test Item"}), &ctx)
        .await;
    let id = id.expect("Create failed");

    // Verify it exists
    assert!(data_engine.read("Item", &id).await.unwrap().is_some());

    // 5. Execute Action
    let mut params = HashMap::new();
    params.insert("id".to_string(), id.clone());

    let result = action_engine.execute("DeleteItem", params, &data_engine).await;
    assert!(result.is_ok());

    // 6. Verify Deletion
    assert!(data_engine.read("Item", &id).await.unwrap().is_none());
}
