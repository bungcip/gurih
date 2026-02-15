use gurih_ir::{
    EntitySchema, Expression, FieldType, PostingLineSchema, PostingRuleSchema, Schema, StateSchema, Symbol, Transition,
    TransitionEffect, WorkflowSchema, FieldSchema,
};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::MemoryDataStore;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

fn create_field(name: &str, ftype: FieldType) -> FieldSchema {
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

#[tokio::test]
async fn bench_posting_rule_performance() {
    let mut schema = Schema::default();

    // 1. Define Entities
    schema.entities.insert(
        Symbol::from("JournalEntry"),
        EntitySchema {
            name: Symbol::from("JournalEntry"),
            table_name: Symbol::from("journal_entry"),
            fields: vec![
                create_field("id", FieldType::Pk),
                create_field("description", FieldType::String),
                create_field("date", FieldType::Date),
                create_field("status", FieldType::String),
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    schema.entities.insert(
        Symbol::from("JournalLine"),
        EntitySchema {
            name: Symbol::from("JournalLine"),
            table_name: Symbol::from("journal_line"),
            fields: vec![
                create_field("id", FieldType::Pk),
                create_field("account", FieldType::String),
                create_field("debit", FieldType::Money),
                create_field("credit", FieldType::Money),
                create_field("journal_entry", FieldType::String),
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    schema.entities.insert(
        Symbol::from("Account"),
        EntitySchema {
            name: Symbol::from("Account"),
            table_name: Symbol::from("account"),
            fields: vec![
                create_field("id", FieldType::Pk),
                create_field("code", FieldType::String),
                create_field("name", FieldType::String),
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // 2. Define Posting Rule with many lines
    let line_count = 1000;
    let mut lines = Vec::with_capacity(line_count);

    // We'll alternate between Account1 (Debit) and Account2 (Credit)
    for i in 0..line_count {
        if i % 2 == 0 {
            lines.push(PostingLineSchema {
                account: Symbol::from("1001"), // Will lookup by code
                debit_expr: Some(Expression::Literal(10.0)),
                credit_expr: None,
            });
        } else {
            lines.push(PostingLineSchema {
                account: Symbol::from("2001"),
                debit_expr: None,
                credit_expr: Some(Expression::Literal(10.0)),
            });
        }
    }

    schema.posting_rules.insert(
        Symbol::from("HeavyRule"),
        PostingRuleSchema {
            name: Symbol::from("HeavyRule"),
            source_entity: Symbol::from("JournalEntry"),
            description_expr: Expression::StringLiteral("Heavy Posting".to_string()),
            date_expr: Expression::StringLiteral("2023-01-01".to_string()),
            lines,
        },
    );

    // 3. Define Workflow
    schema.workflows.insert(
        Symbol::from("JournalEntry"),
        WorkflowSchema {
            name: Symbol::from("JournalEntry"),
            entity: Symbol::from("JournalEntry"),
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
                preconditions: vec![],
                effects: vec![TransitionEffect::Custom {
                    name: Symbol::from("post_journal"),
                    args: vec![Expression::StringLiteral("HeavyRule".to_string())],
                    kwargs: HashMap::new(),
                }],
            }],
        },
    );

    // Use MemoryDataStore for CI speed, but real performance gain is visible with Sqlite/Postgres
    let ds = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(Arc::new(schema), ds.clone()).with_plugins(vec![Box::new(FinancePlugin)]);
    let ctx = RuntimeContext::system();

    // 4. Create Accounts
    engine.create("Account", json!({
        "code": "1001",
        "name": "Cash"
    }), &ctx).await.unwrap();

    engine.create("Account", json!({
        "code": "2001",
        "name": "Revenue"
    }), &ctx).await.unwrap();

    // 5. Create Journal Entry (Draft)
    let je_id = engine.create("JournalEntry", json!({
        "description": "Test Entry",
        "date": "2023-01-01",
        "status": "Draft"
    }), &ctx).await.unwrap();

    // 6. Benchmark Transition
    println!("Starting benchmark for {} lines...", line_count);
    let start = Instant::now();

    engine.update("JournalEntry", &je_id, json!({
        "status": "Posted"
    }), &ctx).await.unwrap();

    let duration = start.elapsed();
    println!("BENCH_RESULT: Posting {} lines took: {:?}", line_count, duration);
}
