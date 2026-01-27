use gurih_ir::{ActionLogic, ActionStep, EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_runtime::action::ActionEngine;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::MemoryDataStore;
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
            name: Symbol::from("id"),
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
        Symbol::from("Item"),
        EntitySchema {
            name: Symbol::from("Item"), // Missing in previous test
            table_name: Symbol::from("item"),
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
        Symbol::from("DeleteItem"),
        ActionLogic {
            name: Symbol::from("DeleteItem"),
            params: vec![Symbol::from("id")],
            steps: vec![ActionStep {
                step_type: gurih_ir::ActionStepType::EntityDelete,
                target: Symbol::from("Item"),
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
    let datastore = Arc::new(MemoryDataStore::new());
    let data_engine = DataEngine::new(schema_arc.clone(), datastore.clone());
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

    let result = action_engine.execute("DeleteItem", params, &data_engine, &ctx).await;
    assert!(result.is_ok());

    // 6. Verify Deletion
    assert!(data_engine.read("Item", &id).await.unwrap().is_none());
}
