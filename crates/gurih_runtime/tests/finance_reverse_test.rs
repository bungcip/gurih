use gurih_ir::{ActionLogic, ActionStep, ActionStepType, EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_runtime::action::ActionEngine;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::{DataStore, MemoryDataStore};
use gurih_runtime::plugins::FinancePlugin;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_finance_reverse_journal() {
    // 1. Setup Schema
    let mut entities = HashMap::new();

    // JournalEntry
    let je_fields = vec![
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
            name: Symbol::from("entry_number"),
            field_type: FieldType::Serial,
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
            name: Symbol::from("description"),
            field_type: FieldType::Text,
            required: false,
            unique: false,
            default: None,
            references: None,
            serial_generator: None,
            storage: None,
            resize: None,
            filetype: None,
        },
        FieldSchema {
            name: Symbol::from("status"),
            field_type: FieldType::String,
            required: false,
            unique: false,
            default: Some("Draft".to_string()),
            references: None,
            serial_generator: None,
            storage: None,
            resize: None,
            filetype: None,
        },
        FieldSchema {
            name: Symbol::from("related_journal"),
            field_type: FieldType::String,
            required: false,
            unique: false,
            default: None,
            references: Some(Symbol::from("JournalEntry")),
            serial_generator: None,
            storage: None,
            resize: None,
            filetype: None,
        },
    ];
    entities.insert(
        Symbol::from("JournalEntry"),
        EntitySchema {
            name: Symbol::from("JournalEntry"),
            table_name: Symbol::from("journal_entry"),
            fields: je_fields,
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // JournalLine
    let jl_fields = vec![
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
            name: Symbol::from("journal_entry"),
            field_type: FieldType::String,
            required: true,
            unique: false,
            default: None,
            references: Some(Symbol::from("JournalEntry")),
            serial_generator: None,
            storage: None,
            resize: None,
            filetype: None,
        },
        FieldSchema {
            name: Symbol::from("debit"),
            field_type: FieldType::Money,
            required: false,
            unique: false,
            default: None,
            references: None,
            serial_generator: None,
            storage: None,
            resize: None,
            filetype: None,
        },
        FieldSchema {
            name: Symbol::from("credit"),
            field_type: FieldType::Money,
            required: false,
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
        Symbol::from("JournalLine"),
        EntitySchema {
            name: Symbol::from("JournalLine"),
            table_name: Symbol::from("journal_line"),
            fields: jl_fields,
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // 2. Setup Action
    let mut actions = HashMap::new();
    let mut args = HashMap::new();
    args.insert("id".to_string(), "param(\"id\")".to_string());

    actions.insert(
        Symbol::from("ReverseJournal"),
        ActionLogic {
            name: Symbol::from("ReverseJournal"),
            params: vec![Symbol::from("id")],
            steps: vec![ActionStep {
                step_type: ActionStepType::Custom("finance:reverse_journal".to_string()),
                target: Symbol::from("JournalEntry"),
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

    // 3. Engines
    let datastore = Arc::new(MemoryDataStore::new());
    let data_engine = DataEngine::new(schema_arc.clone(), datastore.clone());
    let action_engine = ActionEngine::new(actions).with_plugins(vec![Box::new(FinancePlugin)]);
    let ctx = RuntimeContext::system();

    // 4. Create Data
    // Journal
    let je_id = data_engine
        .create(
            "JournalEntry",
            json!({
                "id": "je-1",
                "entry_number": "JE/001",
                "description": "Original Entry",
                "status": "Posted"
            }),
            &ctx,
        )
        .await
        .expect("Create JE failed");

    // Lines
    data_engine
        .create(
            "JournalLine",
            json!({
                "id": "jl-1",
                "journal_entry": je_id,
                "debit": "100.0",
                "credit": "0.0"
            }),
            &ctx,
        )
        .await
        .expect("Create JL1 failed");

    data_engine
        .create(
            "JournalLine",
            json!({
                "id": "jl-2",
                "journal_entry": je_id,
                "debit": "0.0",
                "credit": "100.0"
            }),
            &ctx,
        )
        .await
        .expect("Create JL2 failed");

    // 5. Execute Action
    let mut params = HashMap::new();
    params.insert("id".to_string(), je_id.clone());

    action_engine
        .execute("ReverseJournal", params, &data_engine, &ctx)
        .await
        .expect("Action execution failed");

    // 6. Verify
    // Find all JournalEntries
    let journals = data_engine
        .list("JournalEntry", None, None, None)
        .await
        .expect("List failed");
    assert_eq!(journals.len(), 2);

    let reversal = journals
        .iter()
        .find(|j| j["id"].as_str() != Some("je-1"))
        .expect("Reversal not found");
    assert_eq!(reversal["description"], "Reversal of JE/001");
    assert_eq!(reversal["status"], "Draft");
    assert_eq!(reversal["related_journal"], je_id);

    // Verify Lines
    let reversal_id = reversal["id"].as_str().unwrap();
    let mut filters = HashMap::new();
    filters.insert("journal_entry".to_string(), reversal_id.to_string());
    let lines = datastore
        .find("journal_line", filters)
        .await
        .expect("Find lines failed");
    assert_eq!(lines.len(), 2);

    // Helper to parse money string
    let parse_money = |v: &serde_json::Value| -> f64 { v.as_str().unwrap().parse().unwrap() };

    let l1 = lines
        .iter()
        .find(|l| parse_money(&l["debit"]) == 0.0 && parse_money(&l["credit"]) == 100.0);
    assert!(
        l1.is_some(),
        "Should find a line with Debit 0, Credit 100 (Reversal of JL1)"
    );

    let l2 = lines
        .iter()
        .find(|l| parse_money(&l["debit"]) == 100.0 && parse_money(&l["credit"]) == 0.0);
    assert!(
        l2.is_some(),
        "Should find a line with Debit 100, Credit 0 (Reversal of JL2)"
    );
}
