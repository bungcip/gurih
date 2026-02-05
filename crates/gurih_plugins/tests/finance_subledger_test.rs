use gurih_ir::{Schema, StateSchema, Symbol, Transition, TransitionPrecondition, WorkflowSchema};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::MemoryDataStore;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

fn create_subledger_schema() -> Schema {
    let mut schema = Schema::default();
    let entity_name = Symbol::from("JournalEntry");
    let line_entity = Symbol::from("JournalLine");
    let account_entity = Symbol::from("Account");
    let customer_entity = Symbol::from("Customer");

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

    // Account
    schema.entities.insert(
        account_entity,
        gurih_ir::EntitySchema {
            name: account_entity,
            table_name: Symbol::from("account"),
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
                    name: Symbol::from("code"),
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
                    name: Symbol::from("name"),
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
                    name: Symbol::from("requires_party"),
                    field_type: gurih_ir::FieldType::Boolean,
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

    // Customer
    schema.entities.insert(
        customer_entity,
        gurih_ir::EntitySchema {
            name: customer_entity,
            table_name: Symbol::from("customer"),
            fields: vec![
                gurih_ir::FieldSchema {
                    name: Symbol::from("id"),
                    field_type: gurih_ir::FieldType::String, // Usually UUID
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
                    name: Symbol::from("name"),
                    field_type: gurih_ir::FieldType::Name,
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
                    name: Symbol::from("account"),
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
                // New Fields
                gurih_ir::FieldSchema {
                    name: Symbol::from("party_type"),
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
                    name: Symbol::from("party_id"),
                    field_type: gurih_ir::FieldType::Uuid,
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
            preconditions: vec![
                TransitionPrecondition::Custom {
                    name: Symbol::from("balanced_transaction"),
                    args: vec![],
                    kwargs: HashMap::new(),
                },
                TransitionPrecondition::Custom {
                    name: Symbol::from("valid_parties"),
                    args: vec![],
                    kwargs: HashMap::new(),
                },
            ],
            effects: vec![],
        }],
    };
    schema.workflows.insert(workflow.name, workflow);
    schema
}

#[tokio::test]
async fn test_subledger_validation() {
    let schema = create_subledger_schema();
    let datastore = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(Arc::new(schema), datastore).with_plugins(vec![Box::new(FinancePlugin)]);
    let ctx = RuntimeContext::system();

    // 1. Create Accounts
    let ar_account_id = engine
        .create(
            "Account",
            json!({
                "code": "101",
                "requires_party": true
            }),
            &ctx,
        )
        .await
        .expect("AR Account create failed");

    let cash_account_id = engine
        .create(
            "Account",
            json!({
                "code": "100",
                "requires_party": false
            }),
            &ctx,
        )
        .await
        .expect("Cash Account create failed");

    // 2. Create Customer
    let customer_id = engine
        .create(
            "Customer",
            json!({
                "name": "Acme Corp"
            }),
            &ctx,
        )
        .await
        .expect("Customer create failed");

    // --- CASE 1: Posting to AR without Party (Fail) ---
    let j1 = engine
        .create("JournalEntry", json!({ "status": "Draft", "date": "2024-01-01" }), &ctx)
        .await
        .unwrap();

    // Debit AR (missing party)
    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": j1,
                "account": ar_account_id,
                "debit": "100",
                "credit": "0"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // Credit Cash
    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": j1,
                "account": cash_account_id,
                "debit": "0",
                "credit": "100"
            }),
            &ctx,
        )
        .await
        .unwrap();

    let res = engine
        .update("JournalEntry", &j1, json!({"status": "Posted"}), &ctx)
        .await;
    assert!(res.is_err(), "Should fail because AR requires party");
    let err = res.err().unwrap();
    println!("Err1: {}", err);
    assert!(err.contains("requires a Party"));

    // --- CASE 2: Posting to AR with Invalid Party (Fail) ---
    let j2 = engine
        .create("JournalEntry", json!({ "status": "Draft", "date": "2024-01-01" }), &ctx)
        .await
        .unwrap();

    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": j2,
                "account": ar_account_id,
                "debit": "100",
                "credit": "0",
                "party_type": "Customer",
                "party_id": Uuid::new_v4().to_string() // Non-existent
            }),
            &ctx,
        )
        .await
        .unwrap();

    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": j2,
                "account": cash_account_id,
                "debit": "0",
                "credit": "100"
            }),
            &ctx,
        )
        .await
        .unwrap();

    let res = engine
        .update("JournalEntry", &j2, json!({"status": "Posted"}), &ctx)
        .await;
    assert!(res.is_err(), "Should fail because Party does not exist");
    let err = res.err().unwrap();
    println!("Err2: {}", err);
    assert!(err.contains("does not exist"));

    // --- CASE 3: Posting to AR with Valid Party (Success) ---
    let j3 = engine
        .create("JournalEntry", json!({ "status": "Draft", "date": "2024-01-01" }), &ctx)
        .await
        .unwrap();

    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": j3,
                "account": ar_account_id,
                "debit": "100",
                "credit": "0",
                "party_type": "Customer",
                "party_id": customer_id
            }),
            &ctx,
        )
        .await
        .unwrap();

    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": j3,
                "account": cash_account_id,
                "debit": "0",
                "credit": "100"
            }),
            &ctx,
        )
        .await
        .unwrap();

    let res = engine
        .update("JournalEntry", &j3, json!({"status": "Posted"}), &ctx)
        .await;
    assert!(res.is_ok(), "Should pass validation: {:?}", res.err());
}
