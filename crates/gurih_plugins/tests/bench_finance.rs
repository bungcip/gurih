use gurih_ir::{ActionStep, ActionStepType, EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::plugins::Plugin;
use gurih_runtime::store::init_datastore;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

fn create_field(name: &str, field_type: FieldType, pk: bool) -> FieldSchema {
    let mut field = FieldSchema {
        name: Symbol::from(name),
        field_type,
        required: false,
        unique: false,
        default: None,
        references: None,
        serial_generator: None,
        storage: None,
        resize: None,
        filetype: None,
    };
    if pk {
        field.required = true;
        field.unique = true;
    }
    field
}

#[tokio::test]
async fn bench_reverse_journal() {
    // 1. Setup Schema
    let mut schema = Schema {
        database: Some(gurih_ir::DatabaseSchema {
            db_type: gurih_ir::DatabaseType::Sqlite,
            url: "sqlite::memory:".to_string(),
        }),
        ..Default::default()
    };

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

#[tokio::test]
async fn bench_generate_closing_entry() {
    // 1. Setup Schema
    let mut schema = Schema {
        database: Some(gurih_ir::DatabaseSchema {
            db_type: gurih_ir::DatabaseType::Sqlite,
            url: "sqlite::memory:".to_string(),
        }),
        ..Default::default()
    };

    // Account
    schema.entities.insert(
        Symbol::from("Account"),
        EntitySchema {
            name: Symbol::from("Account"),
            table_name: Symbol::from("account"),
            fields: vec![
                create_field("id", FieldType::String, true),
                create_field("name", FieldType::String, false),
                create_field("type", FieldType::String, false),
                create_field("system_tag", FieldType::String, false),
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // JournalEntry
    schema.entities.insert(
        Symbol::from("JournalEntry"),
        EntitySchema {
            name: Symbol::from("JournalEntry"),
            table_name: Symbol::from("journal_entry"),
            fields: vec![
                create_field("id", FieldType::String, true),
                create_field("description", FieldType::String, false),
                create_field("date", FieldType::Date, false),
                create_field("status", FieldType::String, false),
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // JournalLine
    schema.entities.insert(
        Symbol::from("JournalLine"),
        EntitySchema {
            name: Symbol::from("JournalLine"),
            table_name: Symbol::from("journal_line"),
            fields: vec![
                create_field("id", FieldType::String, true),
                create_field("journal_entry", FieldType::String, false),
                create_field("account", FieldType::String, false),
                create_field("debit", FieldType::Money, false),
                create_field("credit", FieldType::Money, false),
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // AccountingPeriod
    schema.entities.insert(
        Symbol::from("AccountingPeriod"),
        EntitySchema {
            name: Symbol::from("AccountingPeriod"),
            table_name: Symbol::from("accounting_period"),
            fields: vec![
                create_field("id", FieldType::String, true),
                create_field("name", FieldType::String, false),
                create_field("start_date", FieldType::Date, false),
                create_field("end_date", FieldType::Date, false),
                create_field("status", FieldType::String, false),
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    let schema_arc = Arc::new(schema);

    // 2. Init Datastore & Engine
    let datastore = init_datastore(schema_arc.clone(), None)
        .await
        .expect("Failed to init datastore");

    let engine = DataEngine::new(schema_arc.clone(), datastore);
    let ctx = RuntimeContext::system();
    let plugin = FinancePlugin;

    // 3. Create Data
    // Period
    let period_id = engine
        .create(
            "AccountingPeriod",
            json!({
                "name": "2024",
                "start_date": "2024-01-01",
                "end_date": "2024-12-31",
                "status": "Open"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // Retained Earnings
    engine
        .create(
            "Account",
            json!({ "name": "Retained Earnings", "type": "Equity", "system_tag": "retained_earnings" }),
            &ctx,
        )
        .await
        .unwrap();

    // Create many Revenue/Expense Accounts and post entries
    let num_accounts = 500;
    let mut journal_lines = Vec::new();

    let journal_id = engine
        .create(
            "JournalEntry",
            json!({
                "description": "Daily Sales",
                "date": "2024-06-01",
                "status": "Posted"
            }),
            &ctx,
        )
        .await
        .unwrap();

    for i in 0..num_accounts {
        let acc_type = if i % 2 == 0 { "Revenue" } else { "Expense" };
        let acc_id = engine
            .create(
                "Account",
                json!({ "name": format!("Account {}", i), "type": acc_type }),
                &ctx,
            )
            .await
            .unwrap();

        let (debit, credit) = if acc_type == "Revenue" {
            ("0.00", "100.00")
        } else {
            ("50.00", "0.00")
        };

        journal_lines.push(json!({
            "journal_entry": journal_id,
            "account": acc_id,
            "debit": debit,
            "credit": credit
        }));
    }

    // Use create_many for setup speed (assuming it works, otherwise loop)
    engine.create_many("JournalLine", journal_lines, &ctx).await.unwrap();

    // 4. Measure Closing Entry
    let start = Instant::now();

    let step = ActionStep {
        step_type: ActionStepType::Custom("finance:generate_closing_entry".to_string()),
        target: Symbol::from("JournalEntry"),
        args: HashMap::from([("period_id".to_string(), period_id.clone())]),
    };

    let params = HashMap::new();
    let result = plugin
        .execute_action_step("finance:generate_closing_entry", &step, &params, &engine, &ctx)
        .await
        .expect("Failed to generate closing entry");

    let duration = start.elapsed();
    println!(
        "Generate Closing Entry ({} accounts) took: {:?}",
        num_accounts, duration
    );

    assert!(result);
}
