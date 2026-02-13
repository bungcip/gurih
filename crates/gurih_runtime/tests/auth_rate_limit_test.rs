use gurih_ir::{EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_runtime::auth::AuthEngine;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::MemoryDataStore;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_rate_limiting() {
    // 1. Setup
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
            name: Symbol::from("password"),
            field_type: FieldType::Password,
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
        Symbol::from("User"),
        EntitySchema {
            name: Symbol::from("User"),
            table_name: Symbol::from("user"),
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

    let datastore = Arc::new(MemoryDataStore::new());
    let data_engine = DataEngine::new(schema_arc.clone(), datastore.clone());
    let auth_engine = AuthEngine::new(datastore.clone(), Some("user".to_string()), Some(schema_arc.clone()));

    let ctx = RuntimeContext::system();
    data_engine
        .create(
            "User",
            json!({
                "username": "victim",
                "password": "securepassword"
            }),
            &ctx,
        )
        .await
        .expect("Failed to create user");

    // 2. Attack: Try 10 times with wrong password
    let mut blocked = false;
    for i in 1..=10 {
        let res = auth_engine.login("victim", "wrongpassword").await;
        if let Err(msg) = res {
            println!("Attempt {}: {}", i, msg);
            if msg.contains("Too many failed attempts") {
                blocked = true;
                break;
            }
        }
    }

    assert!(blocked, "Should have been rate limited after multiple failed attempts");
}
