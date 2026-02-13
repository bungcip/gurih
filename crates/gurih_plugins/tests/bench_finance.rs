use gurih_ir::{ActionStep, ActionStepType, EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::init_datastore;
use gurih_runtime::plugins::Plugin;
use gurih_runtime::traits::DataAccess;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[tokio::test]
async fn bench_reverse_journal() {
    // 1. Setup Schema
    let mut schema = Schema::default();
    schema.database = Some(gurih_ir::DatabaseSchema {
        db_type: gurih_ir::DatabaseType::Sqlite,
        url: "sqlite::memory:".to_string(),
    });

    // JournalEntry
    let je = EntitySchema {
        name: Symbol::from("JournalEntry"),
        table_name: Symbol::from("journal_entry"),
        fields: vec![
            FieldSchema {
                name: Symbol::from("id"),
                field_type: FieldType::Uuid,
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
                name: Symbol::from("entry_number"),
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
                name: Symbol::from("related_journal"),
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
        ],
        relationships: vec![],
        options: HashMap::new(),
        seeds: None,
    };
    schema.entities.insert(Symbol::from("JournalEntry"), je);

    // JournalLine
    let jl = EntitySchema {
        name: Symbol::from("JournalLine"),
        table_name: Symbol::from("journal_line"),
        fields: vec![
            FieldSchema {
                name: Symbol::from("id"),
                field_type: FieldType::Uuid,
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
                name: Symbol::from("journal_entry"),
                field_type: FieldType::Uuid,
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
        ],
        relationships: vec![],
        options: HashMap::new(),
        seeds: None,
    };
    schema.entities.insert(Symbol::from("JournalLine"), jl);

    let schema_arc = Arc::new(schema);

    // 2. Init Datastore & Engine
    let datastore = init_datastore(schema_arc.clone(), None)
        .await
        .expect("Failed to init datastore");

    let engine = DataEngine::new(schema_arc.clone(), datastore.clone());
    let plugin = FinancePlugin;
    let ctx = RuntimeContext {
        user_id: "test-user".to_string(),
        roles: vec!["admin".to_string()],
        token: None,
        permissions: vec!["*".to_string()],
    };

    // 3. Create Data
    // Create 1 Journal Entry with N lines
    let journal_id = engine
        .create(
            "JournalEntry",
            json!({
                "description": "Original Journal",
                "status": "Posted",
                "entry_number": "JE-001"
            }),
            &ctx,
        )
        .await
        .expect("Failed to create JE");

    let num_lines = 500;
    for i in 0..num_lines {
        engine
            .create(
                "JournalLine",
                json!({
                    "journal_entry": journal_id,
                    "debit": if i % 2 == 0 { "100.00" } else { "0.00" },
                    "credit": if i % 2 != 0 { "100.00" } else { "0.00" }
                }),
                &ctx,
            )
            .await
            .expect("Failed to create JL");
    }

    // 4. Measure
    let start = Instant::now();

    let step = ActionStep {
        step_type: ActionStepType::Custom("reverse".to_string()),
        target: Symbol::from("JournalEntry"),
        args: {
            let mut map = HashMap::new();
            map.insert("id".to_string(), journal_id.clone());
            map
        },
    };

    let params = HashMap::new();

    let result = plugin
        .execute_action_step("finance:reverse_journal", &step, &params, &engine, &ctx)
        .await
        .expect("Failed to execute reverse journal");

    let duration = start.elapsed();
    println!("Reverse Journal ({} lines) took: {:?}", num_lines, duration);

    assert!(result);
}
