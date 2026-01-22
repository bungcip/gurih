use gurih_ir::{EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_runtime::auth::AuthEngine;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::MemoryDataStore;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_password_hashing() {
    // 1. Setup Schema with "User" entity
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
            field_type: FieldType::Password, // Important!
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

    // 2. Setup Engines
    let datastore = Arc::new(MemoryDataStore::new());
    let data_engine = DataEngine::new(schema_arc.clone(), datastore.clone());
    let auth_engine = AuthEngine::new(datastore.clone());

    // 3. Create User
    let ctx = RuntimeContext::system();
    let password = "mysecretpassword";
    let id = data_engine
        .create(
            "User",
            json!({
                "username": "admin",
                "password": password
            }),
            &ctx,
        )
        .await
        .expect("Failed to create user");

    // 4. Verify Login (Should work)
    let login_result = auth_engine.login("admin", password).await;
    assert!(login_result.is_ok(), "Login failed with correct password");

    // 5. Verify Stored Password is NOT Plaintext
    let stored_user = data_engine.read("User", &id).await.unwrap().unwrap();
    let stored_pass = stored_user.get("password").unwrap().as_str().unwrap();

    println!("Stored password: {}", stored_pass);
    assert_ne!(stored_pass, password, "Password stored in plaintext!");
    assert_ne!(stored_pass, "", "Password should not be empty");
    assert!(stored_pass.contains('$'), "Password should be salted (contain $)");
}
