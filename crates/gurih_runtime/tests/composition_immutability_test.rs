use gurih_ir::{
    EntitySchema, FieldSchema, FieldType, Ownership, RelationshipSchema, RelationshipType, Schema,
    StateSchema as WorkflowState, Symbol, Transition, WorkflowSchema,
};
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::MemoryDataStore;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

fn default_field(name: &str, ftype: FieldType) -> FieldSchema {
    FieldSchema {
        name: Symbol::from(name),
        field_type: ftype,
        required: false,
        unique: false,
        default: None,
        references: None,
        serial_generator: None,
        storage: None,
        resize: None,
        filetype: None,
    }
}

fn setup_schema() -> Schema {
    let mut schema = Schema::default();

    // Parent Entity
    let parent = EntitySchema {
        name: Symbol::from("Parent"),
        table_name: Symbol::from("parent"),
        fields: vec![
            {
                let mut f = default_field("id", FieldType::Uuid);
                f.required = true;
                f.unique = true;
                f
            },
            default_field("status", FieldType::String),
        ],
        relationships: vec![RelationshipSchema {
            name: Symbol::from("children"),
            target_entity: Symbol::from("Child"),
            rel_type: RelationshipType::HasMany,
            ownership: Ownership::Composition,
        }],
        options: HashMap::new(),
        seeds: None,
    };
    schema.entities.insert(Symbol::from("Parent"), parent);

    // Child Entity
    let child = EntitySchema {
        name: Symbol::from("Child"),
        table_name: Symbol::from("child"),
        fields: vec![
            {
                let mut f = default_field("id", FieldType::Uuid);
                f.required = true;
                f.unique = true;
                f
            },
            {
                let mut f = default_field("parent_id", FieldType::String);
                f.required = true;
                f
            },
            default_field("amount", FieldType::Money),
        ],
        relationships: vec![RelationshipSchema {
            name: Symbol::from("parent"),
            target_entity: Symbol::from("Parent"),
            rel_type: RelationshipType::BelongsTo,
            ownership: Ownership::Composition, // IMPORTANT: Triggers the check
        }],
        options: HashMap::new(),
        seeds: None,
    };
    schema.entities.insert(Symbol::from("Child"), child);

    // Workflow for Parent
    let workflow = WorkflowSchema {
        name: Symbol::from("ParentWorkflow"),
        entity: Symbol::from("Parent"),
        field: Symbol::from("status"),
        initial_state: Symbol::from("Draft"),
        states: vec![
            WorkflowState {
                name: Symbol::from("Draft"),
                immutable: false,
            },
            WorkflowState {
                name: Symbol::from("Posted"),
                immutable: true, // IMPORTANT
            },
        ],
        transitions: vec![Transition {
            name: Symbol::from("post"),
            from: Symbol::from("Draft"),
            to: Symbol::from("Posted"),
            required_permission: None,
            preconditions: vec![],
            effects: vec![],
        }],
    };
    schema.workflows.insert(Symbol::from("ParentWorkflow"), workflow);

    schema
}

#[tokio::test]
async fn test_child_update_fails_if_parent_locked() {
    let schema = setup_schema();
    let datastore = Arc::new(MemoryDataStore::default());
    let engine = DataEngine::new(Arc::new(schema), datastore);
    let ctx = gurih_runtime::context::RuntimeContext::system();

    // 2. Create Parent (Draft)
    let parent_id = engine
        .create("Parent", json!({ "status": "Draft" }), &ctx)
        .await
        .unwrap();

    // 3. Create Child
    let child_id = engine
        .create(
            "Child",
            json!({
                "parent_id": parent_id,
                "amount": "100.00"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // 4. Update Parent to Posted (Immutable)
    engine
        .update("Parent", &parent_id, json!({ "status": "Posted" }), &ctx)
        .await
        .unwrap();

    // 5. Try to Update Child
    let result = engine
        .update("Child", &child_id, json!({ "amount": "999.00" }), &ctx)
        .await;

    assert!(result.is_err(), "Update should have failed");
}

#[tokio::test]
async fn test_create_child_into_locked_parent() {
    let schema = setup_schema();
    let datastore = Arc::new(MemoryDataStore::default());
    let engine = DataEngine::new(Arc::new(schema), datastore);
    let ctx = gurih_runtime::context::RuntimeContext::system();

    // 2. Create Parent (Draft)
    let parent_id = engine
        .create("Parent", json!({ "status": "Draft" }), &ctx)
        .await
        .unwrap();

    // 3. Post Parent
    engine
        .update("Parent", &parent_id, json!({ "status": "Posted" }), &ctx)
        .await
        .unwrap();

    // 4. Try Create Child into Posted Parent
    let result = engine
        .create(
            "Child",
            json!({
                "parent_id": parent_id,
                "amount": "100.00"
            }),
            &ctx,
        )
        .await;

    assert!(result.is_err(), "Should not create child in locked parent");
}

#[tokio::test]
async fn test_move_child_to_locked_parent() {
    let schema = setup_schema();
    let datastore = Arc::new(MemoryDataStore::default());
    let engine = DataEngine::new(Arc::new(schema), datastore);
    let ctx = gurih_runtime::context::RuntimeContext::system();

    // 2. Create Parent 1 (Draft -> Posted)
    let p1_id = engine
        .create("Parent", json!({ "status": "Draft" }), &ctx)
        .await
        .unwrap();
    engine
        .update("Parent", &p1_id, json!({ "status": "Posted" }), &ctx)
        .await
        .unwrap();

    // 3. Create Parent 2 (Draft)
    let p2_id = engine
        .create("Parent", json!({ "status": "Draft" }), &ctx)
        .await
        .unwrap();

    // 4. Create Child in P2
    let child_id = engine
        .create(
            "Child",
            json!({
                "parent_id": p2_id,
                "amount": "100.00"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // 5. Try Move Child from P2 to P1 (Locked)
    let result = engine
        .update("Child", &child_id, json!({ "parent_id": p1_id }), &ctx)
        .await;

    assert!(result.is_err(), "Should not move child to locked parent");
}
