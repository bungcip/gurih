use gurih_ir::{EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::DataStore;
use gurih_runtime::store::MemoryDataStore;
use gurih_runtime::traits::DataAccess;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_access_control_vulnerability() {
    // 1. Setup Schema with User entity
    let mut schema = Schema::default();
    let user_sym = Symbol::from("User");

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
    ];

    let user_entity = EntitySchema {
        name: user_sym.clone(),
        table_name: Symbol::from("users"),
        fields,
        relationships: vec![],
        options: HashMap::new(),
        seeds: None,
    };

    // Default permissions are read:User, update:User, delete:User
    schema.entities.insert(user_sym.clone(), user_entity);

    let schema_arc = Arc::new(schema);
    let datastore = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(schema_arc, datastore.clone());

    // 2. Insert a dummy user directly (bypass permissions for setup)
    datastore
        .insert(
            "users",
            json!({
                "id": "target_user",
                "username": "target"
            }),
        )
        .await
        .unwrap();

    // 3. Context with NO permissions
    let ctx = RuntimeContext {
        user_id: "attacker".to_string(),
        roles: vec!["user".to_string()],
        permissions: vec![], // No permissions!
        token: None,
    };

    // 4. Test Vulnerabilities

    // A. LIST (Should fail now)
    let list_result = engine.list("User", None, None, None, &ctx).await;
    assert!(
        list_result.is_err(),
        "SECURITY FIX VERIFICATION: list() should fail without permission!"
    );
    assert!(list_result.err().unwrap().contains("Missing permission"));

    // B. READ (Should fail now)
    let read_result = engine.read("User", "target_user", &ctx).await;
    assert!(
        read_result.is_err(),
        "SECURITY FIX VERIFICATION: read() should fail without permission!"
    );
    assert!(read_result.err().unwrap().contains("Missing permission"));

    // C. UPDATE (Should fail now)
    let update_result = engine
        .update("User", "target_user", json!({ "username": "pwned" }), &ctx)
        .await;
    assert!(
        update_result.is_err(),
        "SECURITY FIX VERIFICATION: update() should fail without permission!"
    );
    assert!(update_result.err().unwrap().contains("Missing permission"));

    // D. DELETE (Should fail now)
    let delete_result = engine.delete("User", "target_user", &ctx).await;
    assert!(
        delete_result.is_err(),
        "SECURITY FIX VERIFICATION: delete() should fail without permission!"
    );
    assert!(delete_result.err().unwrap().contains("Missing permission"));
}
