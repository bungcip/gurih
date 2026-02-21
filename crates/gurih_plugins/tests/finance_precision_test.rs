use gurih_ir::{Schema, StateSchema, Symbol, Transition, TransitionPrecondition, WorkflowSchema};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::MemoryDataStore;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

fn create_precision_schema() -> Schema {
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
                kwargs: HashMap::new(),
            }],
            effects: vec![],
        }],
    };
    schema.workflows.insert(workflow.name, workflow);
    schema
}

#[tokio::test]
async fn test_decimal_precision_posting() {
    let schema = create_precision_schema();
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

    // 2. Create Lines designed to trigger floating point errors
    // 0.1 + 0.2 = 0.30000000000000004 in f64
    // If we were using f64 with exact equality check, this would fail against 0.3.
    // Since we use Decimal and check is_zero(), it should pass.

    // Debit 0.1
    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": journal_id,
                "debit": "0.1",
                "credit": "0.0"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // Debit 0.2
    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": journal_id,
                "debit": "0.2",
                "credit": "0.0"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // Credit 0.3
    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": journal_id,
                "debit": "0.0",
                "credit": "0.3"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // 3. Post
    let res = engine
        .update("JournalEntry", &journal_id, json!({"status": "Posted"}), &ctx)
        .await;

    assert!(res.is_ok(), "Posting should succeed with decimal arithmetic");
}

#[tokio::test]
async fn test_tiny_imbalance_should_fail() {
    let schema = create_precision_schema();
    let datastore = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(Arc::new(schema), datastore).with_plugins(vec![Box::new(FinancePlugin)]);
    let ctx = RuntimeContext::system();

    let journal_id = engine
        .create("JournalEntry", json!({ "status": "Draft", "date": "2024-01-01" }), &ctx)
        .await
        .unwrap();

    // Debit 100.00
    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": journal_id,
                "debit": "100.00",
                "credit": "0.0"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // Credit 99.99 (Diff 0.01)
    // Previously we checked diff > 0.01, so 0.01 diff might have passed?
    // Now we check is_zero(), so ANY diff fails.
    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": journal_id,
                "debit": "0.0",
                "credit": "99.99"
            }),
            &ctx,
        )
        .await
        .unwrap();

    let res = engine
        .update("JournalEntry", &journal_id, json!({"status": "Posted"}), &ctx)
        .await;

    assert!(res.is_err(), "Tiny imbalance of 0.01 should fail");
}
