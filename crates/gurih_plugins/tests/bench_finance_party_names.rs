use gurih_ir::{EntitySchema, FieldSchema, FieldType, Schema, Symbol};
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
async fn bench_snapshot_parties() {
    let mut schema = Schema {
        database: Some(gurih_ir::DatabaseSchema {
            db_type: gurih_ir::DatabaseType::Sqlite,
            url: "sqlite::memory:".to_string(),
        }),
        ..Default::default()
    };

    schema.entities.insert(
        Symbol::from("JournalEntry"),
        EntitySchema {
            name: Symbol::from("JournalEntry"),
            table_name: Symbol::from("journal_entry"),
            fields: vec![create_field("id", FieldType::String, true)],
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
                create_field("id", FieldType::String, true),
                create_field("journal_entry", FieldType::String, false),
                create_field("party_type", FieldType::String, false),
                create_field("party_id", FieldType::String, false),
                create_field("party_name", FieldType::String, false),
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    schema.entities.insert(
        Symbol::from("Customer"),
        EntitySchema {
            name: Symbol::from("Customer"),
            table_name: Symbol::from("customer"),
            fields: vec![
                create_field("id", FieldType::String, true),
                create_field("name", FieldType::String, false),
                create_field("full_name", FieldType::String, false),
                create_field("description", FieldType::String, false),
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    let schema_arc = Arc::new(schema);
    let datastore = init_datastore(schema_arc.clone(), None).await.unwrap();
    let engine = DataEngine::new(schema_arc.clone(), datastore.clone());
    let ctx = RuntimeContext::system();
    let plugin = FinancePlugin;

    let journal_id = engine.create("JournalEntry", json!({}), &ctx).await.unwrap();

    let num_customers = 1000;
    let mut customer_ids = Vec::new();
    for i in 0..num_customers {
        let cid = engine
            .create("Customer", json!({"name": format!("Customer {}", i)}), &ctx)
            .await
            .unwrap();
        customer_ids.push(cid);
    }

    let mut journal_lines = Vec::new();
    for cid in customer_ids {
        journal_lines.push(json!({
            "journal_entry": journal_id,
            "party_type": "Customer",
            "party_id": cid,
        }));
    }
    engine.create_many("JournalLine", journal_lines, &ctx).await.unwrap();

    let entity_data = json!({"id": journal_id});

    let start = Instant::now();
    plugin
        .apply_effect(
            "snapshot_parties",
            &[],
            &HashMap::new(),
            &schema_arc,
            Some(&datastore),
            "JournalEntry",
            &entity_data,
        )
        .await
        .unwrap();
    let duration = start.elapsed();
    println!("snapshot_parties ({} parties) took: {:?}", num_customers, duration);
}
