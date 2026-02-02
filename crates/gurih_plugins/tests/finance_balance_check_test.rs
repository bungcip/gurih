use gurih_ir::{Schema, StateSchema, Symbol, Transition, TransitionPrecondition, WorkflowSchema};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::MemoryDataStore;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

fn create_test_schema() -> Schema {
    let mut schema = Schema::default();
    let entity_name = Symbol::from("JournalEntry");
    let line_entity = Symbol::from("JournalLine");

    // JournalEntry
    schema.entities.insert(
        entity_name,
        gurih_ir::EntitySchema {
            name: entity_name,
            table_name: Symbol::from("journal_entry"),
            fields: vec![
                gurih_ir::FieldSchema {
                    name: Symbol::from("id"),
                    field_type: gurih_ir::FieldType::String,
                    required: false,
                    unique: true,
                    default: None,
                    references: None,
                    serial_generator: None,
                    storage: None,
                    resize: None,
                    filetype: None,
                },
                gurih_ir::FieldSchema {
                    name: Symbol::from("status"),
                    field_type: gurih_ir::FieldType::String,
                    required: false,
                    unique: false,
                    default: None,
                    references: None,
                    serial_generator: None,
                    storage: None,
                    resize: None,
                    filetype: None,
                },
                gurih_ir::FieldSchema {
                    name: Symbol::from("date"),
                    field_type: gurih_ir::FieldType::Date,
                    required: false,
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
        },
    );

    // JournalLine
    schema.entities.insert(
        line_entity,
        gurih_ir::EntitySchema {
            name: line_entity,
            table_name: Symbol::from("journal_line"),
            fields: vec![
                gurih_ir::FieldSchema {
                    name: Symbol::from("id"),
                    field_type: gurih_ir::FieldType::String,
                    required: false,
                    unique: true,
                    default: None,
                    references: None,
                    serial_generator: None,
                    storage: None,
                    resize: None,
                    filetype: None,
                },
                gurih_ir::FieldSchema {
                    name: Symbol::from("journal_entry"),
                    field_type: gurih_ir::FieldType::String,
                    required: false,
                    unique: false,
                    default: None,
                    references: None,
                    serial_generator: None,
                    storage: None,
                    resize: None,
                    filetype: None,
                },
                gurih_ir::FieldSchema {
                    name: Symbol::from("debit"),
                    field_type: gurih_ir::FieldType::Money,
                    required: false,
                    unique: false,
                    default: None,
                    references: None,
                    serial_generator: None,
                    storage: None,
                    resize: None,
                    filetype: None,
                },
                gurih_ir::FieldSchema {
                    name: Symbol::from("credit"),
                    field_type: gurih_ir::FieldType::Money,
                    required: false,
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
        },
    );

    let workflow = WorkflowSchema {
        name: Symbol::from("JournalWF"),
        entity: entity_name,
        field: Symbol::from("status"),
        initial_state: Symbol::from("Draft"),
        states: vec![
            StateSchema {
                name: Symbol::from("Draft"),
                immutable: false,
            },
            StateSchema {
                name: Symbol::from("Posted"),
                immutable: true,
            },
        ],
        transitions: vec![Transition {
            name: Symbol::from("post"),
            from: Symbol::from("Draft"),
            to: Symbol::from("Posted"),
            required_permission: None,
            preconditions: vec![TransitionPrecondition::Custom {
                name: Symbol::from("balanced_transaction"),
                args: vec![],
            }],
            effects: vec![],
        }],
    };
    schema.workflows.insert(workflow.name, workflow);
    schema
}

#[tokio::test]
async fn test_unbalanced_journal_via_db_should_fail() {
    let schema = create_test_schema();
    let datastore = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(Arc::new(schema), datastore).with_plugins(vec![Box::new(FinancePlugin)]);
    let ctx = RuntimeContext::system();

    // 1. Create Journal (Draft)
    let journal_data = json!({
        "status": "Draft",
        "date": "2024-01-01"
    });
    let journal_id = engine
        .create("JournalEntry", journal_data, &ctx)
        .await
        .expect("Failed to create journal");

    // 2. Create ONE Line (Unbalanced)
    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": journal_id,
                "debit": "100.00",
                "credit": "0.00"
            }),
            &ctx,
        )
        .await
        .expect("Failed to create line");

    // 3. Update Status to Posted (without lines in payload)
    let update_data = json!({ "status": "Posted" });
    let res = engine.update("JournalEntry", &journal_id, update_data, &ctx).await;

    // 4. Assert Failure
    assert!(res.is_err(), "Should fail to post unbalanced transaction");
    let err = res.err().unwrap();
    assert!(
        err.contains("Transaction not balanced"),
        "Unexpected error message: {}",
        err
    );
}

#[tokio::test]
async fn test_balanced_journal_via_db_should_pass() {
    let schema = create_test_schema();
    let datastore = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(Arc::new(schema), datastore).with_plugins(vec![Box::new(FinancePlugin)]);
    let ctx = RuntimeContext::system();

    // 1. Create Journal (Draft)
    let journal_id = engine
        .create(
            "JournalEntry",
            json!({
                "status": "Draft",
                "date": "2024-01-01"
            }),
            &ctx,
        )
        .await
        .expect("Failed to create journal");

    // 2. Create Balanced Lines
    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": journal_id,
                "debit": "100.00",
                "credit": "0.00"
            }),
            &ctx,
        )
        .await
        .expect("Failed to create line 1");

    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": journal_id,
                "debit": "0.00",
                "credit": "100.00"
            }),
            &ctx,
        )
        .await
        .expect("Failed to create line 2");

    // 3. Update Status to Posted (without lines in payload)
    let update_data = json!({ "status": "Posted" });
    let res = engine.update("JournalEntry", &journal_id, update_data, &ctx).await;

    // 4. Assert Success
    assert!(
        res.is_ok(),
        "Should successfully post balanced transaction: {:?}",
        res.err()
    );
}
