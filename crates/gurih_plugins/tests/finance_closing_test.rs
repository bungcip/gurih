use gurih_ir::{
    ActionLogic, ActionStep, ActionStepType, DatabaseSchema, DatabaseType, EntitySchema, FieldSchema, FieldType,
    Schema, Symbol,
};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::init_datastore;
use gurih_runtime::plugins::Plugin; // Import Plugin trait
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

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
async fn test_generate_closing_entry() {
    // 1. Setup Schema
    let mut schema = Schema::default();

    // Database Config (SQLite Memory)
    schema.database = Some(DatabaseSchema {
        db_type: DatabaseType::Sqlite,
        url: "sqlite::memory:".to_string(),
    });

    // Account Entity
    let account_fields = vec![
        create_field("id", FieldType::String, true),
        create_field("name", FieldType::String, false),
        create_field("code", FieldType::String, false),
        create_field("type", FieldType::String, false),
    ];
    schema.entities.insert(
        Symbol::from("Account"),
        EntitySchema {
            name: Symbol::from("Account"),
            table_name: Symbol::from("account"),
            fields: account_fields,
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // JournalEntry Entity
    let je_fields = vec![
        create_field("id", FieldType::String, true),
        create_field("description", FieldType::String, false),
        create_field("date", FieldType::Date, false),
        create_field("status", FieldType::String, false),
    ];
    schema.entities.insert(
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

    // JournalLine Entity
    let jl_fields = vec![
        create_field("id", FieldType::String, true),
        create_field("journal_entry", FieldType::String, false), // FK
        create_field("account", FieldType::String, false),       // FK
        create_field("debit", FieldType::Money, false),
        create_field("credit", FieldType::Money, false),
    ];
    schema.entities.insert(
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

    // AccountingPeriod Entity
    let ap_fields = vec![
        create_field("id", FieldType::String, true),
        create_field("name", FieldType::String, false),
        create_field("start_date", FieldType::Date, false),
        create_field("end_date", FieldType::Date, false),
        create_field("status", FieldType::String, false),
    ];
    schema.entities.insert(
        Symbol::from("AccountingPeriod"),
        EntitySchema {
            name: Symbol::from("AccountingPeriod"),
            table_name: Symbol::from("accounting_period"),
            fields: ap_fields,
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // Action: GenerateClosingEntry
    let mut closing_args = HashMap::new();
    closing_args.insert("period_id".to_string(), "param('id')".to_string());

    let action = ActionLogic {
        name: Symbol::from("GenerateClosingEntry"),
        params: vec![Symbol::from("id")],
        steps: vec![ActionStep {
            step_type: ActionStepType::Custom("finance:generate_closing_entry".to_string()),
            target: Symbol::from("JournalEntry"),
            args: closing_args,
        }],
    };
    schema.actions.insert(action.name.clone(), action);

    let schema_arc = Arc::new(schema);

    // 2. Init Datastore (Runs Migrations)
    let datastore = init_datastore(schema_arc.clone(), None)
        .await
        .expect("Failed to init datastore");

    // 3. Init Engine
    let engine = DataEngine::new(schema_arc.clone(), datastore).with_plugins(vec![Box::new(FinancePlugin)]);
    let ctx = RuntimeContext::system();

    // 4. Seed Data
    // Accounts
    let rev_id = engine
        .create(
            "Account",
            json!({ "name": "Sales Revenue", "code": "400", "type": "Revenue" }),
            &ctx,
        )
        .await
        .unwrap();
    let exp_id = engine
        .create(
            "Account",
            json!({ "name": "Rent Expense", "code": "500", "type": "Expense" }),
            &ctx,
        )
        .await
        .unwrap();
    let re_id = engine
        .create(
            "Account",
            json!({ "name": "Retained Earnings", "code": "300", "type": "Equity" }),
            &ctx,
        )
        .await
        .unwrap();

    // Period
    let period_id = engine
        .create(
            "AccountingPeriod",
            json!({
                "name": "Jan 2024",
                "start_date": "2024-01-01",
                "end_date": "2024-01-31",
                "status": "Open"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // Journal Entry (Revenue: Credit 1000)
    let je1_id = engine
        .create(
            "JournalEntry",
            json!({
                "description": "Sales",
                "date": "2024-01-15",
                "status": "Posted"
            }),
            &ctx,
        )
        .await
        .unwrap();

    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": je1_id,
                "account": rev_id,
                "credit": "1000.0",
                "debit": "0.0"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // Journal Entry (Expense: Debit 400)
    let je2_id = engine
        .create(
            "JournalEntry",
            json!({
                "description": "Rent",
                "date": "2024-01-20",
                "status": "Posted"
            }),
            &ctx,
        )
        .await
        .unwrap();

    engine
        .create(
            "JournalLine",
            json!({
                "journal_entry": je2_id,
                "account": exp_id,
                "debit": "400.0",
                "credit": "0.0"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // 5. Execute Action
    let plugin = FinancePlugin;
    let step = ActionStep {
        step_type: ActionStepType::Custom("finance:generate_closing_entry".to_string()),
        target: Symbol::from("JournalEntry"),
        args: HashMap::from([("period_id".to_string(), "param('id')".to_string())]),
    };

    let mut params = HashMap::new();
    params.insert("id".to_string(), period_id.clone());

    let handled = plugin
        .execute_action_step("finance:generate_closing_entry", &step, &params, &engine, &ctx)
        .await
        .expect("Action execution failed");

    assert!(handled, "Plugin should handle the action step");

    // 6. Verify Results
    // Find the Closing Entry
    let mut filters = HashMap::new();
    filters.insert("description".to_string(), "Closing Entry for Jan 2024".to_string());

    let closing_entries = engine
        .datastore()
        .find("journal_entry", filters)
        .await
        .expect("Failed to find closing entry");

    assert_eq!(closing_entries.len(), 1, "Should create exactly one closing entry");
    let closing_id = closing_entries[0].get("id").unwrap().as_str().unwrap();

    // Check Lines
    let mut line_filters = HashMap::new();
    line_filters.insert("journal_entry".to_string(), closing_id.to_string());

    let lines = engine
        .datastore()
        .find("journal_line", line_filters)
        .await
        .expect("Failed to find lines");

    assert_eq!(lines.len(), 3, "Should have 3 lines (Rev, Exp, RE)");

    let mut found_rev = false;
    let mut found_exp = false;
    let mut found_re = false;

    for line in lines {
        let acc_id = line.get("account").unwrap().as_str().unwrap();

        // Helper to handle Number or String
        let get_val = |v: Option<&serde_json::Value>| -> f64 {
            if let Some(val) = v {
                if let Some(n) = val.as_f64() {
                    return n;
                }
                if let Some(s) = val.as_str() {
                    return s.parse().unwrap_or(0.0);
                }
            }
            0.0
        };

        let debit = get_val(line.get("debit"));
        let credit = get_val(line.get("credit"));

        if acc_id == rev_id {
            // Revenue was Credit 1000, so Closing should Debit 1000
            assert!((debit - 1000.0).abs() < 0.01, "Revenue should be debited by 1000");
            assert!((credit - 0.0).abs() < 0.01);
            found_rev = true;
        } else if acc_id == exp_id {
            // Expense was Debit 400, so Closing should Credit 400
            assert!((credit - 400.0).abs() < 0.01, "Expense should be credited by 400");
            assert!((debit - 0.0).abs() < 0.01);
            found_exp = true;
        } else if acc_id == re_id {
            // Net Profit = 1000 - 400 = 600.
            // Profit is Credit to Equity.
            assert!(
                (credit - 600.0).abs() < 0.01,
                "Retained Earnings should be credited by 600"
            );
            assert!((debit - 0.0).abs() < 0.01);
            found_re = true;
        }
    }

    assert!(found_rev, "Revenue line missing");
    assert!(found_exp, "Expense line missing");
    assert!(found_re, "Retained Earnings line missing");
}
