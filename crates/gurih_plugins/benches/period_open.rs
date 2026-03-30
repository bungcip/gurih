use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use gurih_ir::{EntitySchema, Expression, FieldSchema, FieldType, Schema, Symbol};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::plugins::Plugin;
use gurih_runtime::store::{DataStore, MemoryDataStore};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn bench_period_open(c: &mut Criterion) {
    let mut schema = Schema::default();

    fn field(name: &str, ftype: FieldType) -> FieldSchema {
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

    schema.entities.insert(
        Symbol::from("AccountingPeriod"),
        EntitySchema {
            name: Symbol::from("AccountingPeriod"),
            table_name: Symbol::from("accounting_period"),
            fields: vec![
                field("id", FieldType::Pk),
                field("status", FieldType::String),
                field("start_date", FieldType::Date),
                field("end_date", FieldType::Date),
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    let rt = Runtime::new().unwrap();

    let datastore = Arc::new(MemoryDataStore::new());
    let ds: Arc<dyn DataStore> = datastore.clone();

    // Insert 1000 dummy periods, none of them open and matching
    rt.block_on(async {
        for i in 0..1000 {
            let period = json!({
                "id": format!("p{}", i),
                "status": if i == 999 { "Open" } else { "Closed" },
                "start_date": format!("2024-{:02}-{:02}", (i % 12) + 1, (i % 28) + 1),
                "end_date": format!("2024-{:02}-{:02}", (i % 12) + 1, (i % 28) + 2)
            });
            ds.insert("accounting_period", period).await.unwrap();
        }

        // Add the correct open period
        let period = json!({
            "id": "open_period",
            "status": "Open",
            "start_date": "2024-01-01",
            "end_date": "2024-12-31"
        });
        ds.insert("accounting_period", period).await.unwrap();
    });

    let plugin = FinancePlugin;
    let schema_arc = Arc::new(schema);
    let entity_data = json!({
        "date": "2024-06-15"
    });

    let args = vec![Expression::StringLiteral("AccountingPeriod".to_string())];
    let kwargs = HashMap::new();

    c.bench_function("finance_period_open_1000", |b| {
        b.to_async(&rt).iter(|| async {
            plugin
                .check_precondition(
                    "period_open",
                    &args,
                    &kwargs,
                    &entity_data,
                    &schema_arc,
                    Some(&ds),
                )
                .await
        });
    });
}

criterion_group!(benches, bench_period_open);
criterion_main!(benches);
