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

#[tokio::test]
async fn test_action_param_with_single_quotes() {
    // Test that param() with single quotes works
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
            name: Symbol::from("Item"),
            table_name: Symbol::from("item"),
            fields,
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // 2. Setup Action Definition with single quotes
    let mut actions = HashMap::new();
    let mut args = HashMap::new();
    args.insert("id".to_string(), "param('id')".to_string());

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
    let id = data_engine
        .create("Item", json!({"id": "item-1", "name": "Test Item"}), &ctx)
        .await;
    let id = id.expect("Create failed");

    // Verify it exists
    assert!(data_engine.read("Item", &id).await.unwrap().is_some());

    // 5. Execute Action with single quotes in param()
    let mut params = HashMap::new();
    params.insert("id".to_string(), id.clone());

    let result = action_engine.execute("DeleteItem", params, &data_engine, &ctx).await;
    assert!(result.is_ok());

    // 6. Verify Deletion
    assert!(data_engine.read("Item", &id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_action_execution_update() {
    // 1. Setup minimal Schema with "Product" entity
    let mut entities = HashMap::new();

    // Entity: Product
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
        FieldSchema {
            name: Symbol::from("price"),
            field_type: FieldType::Float,
            required: true,
            unique: false,
            default: None,
            references: None,
            serial_generator: None,
            storage: None,
            resize: None,
            filetype: None,
        },
        FieldSchema {
            name: Symbol::from("active"),
            field_type: FieldType::Boolean,
            required: true,
            unique: false,
            default: None,
            references: None,
            serial_generator: None,
            storage: None,
            resize: None,
            filetype: None,
        },
        FieldSchema {
            name: Symbol::from("stock"),
            field_type: FieldType::Integer,
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
        Symbol::from("Product"),
        EntitySchema {
            name: Symbol::from("Product"),
            table_name: Symbol::from("product"),
            fields,
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // 2. Setup Action Definition
    let mut actions = HashMap::new();
    let mut args = HashMap::new();
    args.insert("id".to_string(), "param('id')".to_string());
    args.insert("name".to_string(), "param('name')".to_string());
    args.insert("price".to_string(), "param('price')".to_string());
    args.insert("active".to_string(), "param('active')".to_string());
    args.insert("stock".to_string(), "param('stock')".to_string());

    actions.insert(
        Symbol::from("UpdateProduct"),
        ActionLogic {
            name: Symbol::from("UpdateProduct"),
            params: vec![
                Symbol::from("id"),
                Symbol::from("name"),
                Symbol::from("price"),
                Symbol::from("active"),
                Symbol::from("stock"),
            ],
            steps: vec![ActionStep {
                step_type: gurih_ir::ActionStepType::EntityUpdate,
                target: Symbol::from("Product"),
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
    let id = data_engine
        .create("Product", json!({
            "id": "prod-1",
            "name": "Old Product",
            "price": 10.0,
            "active": false,
            "stock": 5
        }), &ctx)
        .await;
    let id = id.expect("Create failed");

    // Verify initial state
    let initial = data_engine.read("Product", &id).await.unwrap().unwrap();
    assert_eq!(initial.get("name").unwrap().as_str().unwrap(), "Old Product");
    assert_eq!(initial.get("price").unwrap().as_f64().unwrap(), 10.0);
    assert_eq!(initial.get("active").unwrap().as_bool().unwrap(), false);
    assert_eq!(initial.get("stock").unwrap().as_i64().unwrap(), 5);

    // 5. Execute Update Action
    let mut params = HashMap::new();
    params.insert("id".to_string(), id.clone());
    params.insert("name".to_string(), "New Product".to_string());
    params.insert("price".to_string(), "15.5".to_string()); // String representation of float
    params.insert("active".to_string(), "true".to_string()); // String representation of bool
    params.insert("stock".to_string(), "20".to_string()); // String representation of int

    let result = action_engine.execute("UpdateProduct", params, &data_engine, &ctx).await;
    assert!(result.is_ok(), "Action failed: {:?}", result.err());

    // 6. Verify Update
    let updated = data_engine.read("Product", &id).await.unwrap().unwrap();
    assert_eq!(updated.get("name").unwrap().as_str().unwrap(), "New Product");
    assert_eq!(updated.get("price").unwrap().as_f64().unwrap(), 15.5);
    assert_eq!(updated.get("active").unwrap().as_bool().unwrap(), true);
    assert_eq!(updated.get("stock").unwrap().as_i64().unwrap(), 20);
}
