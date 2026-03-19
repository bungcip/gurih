use gurih_ir::{EntitySchema, Expression, FieldSchema, FieldType, Schema, Symbol};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::plugins::Plugin;
use gurih_runtime::store::{DataStore, MemoryDataStore};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_finance_period_open_memory_store() {
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

    let datastore = Arc::new(MemoryDataStore::new());
    let ds: Arc<dyn DataStore> = datastore.clone();

    // Insert an open period
    let period = json!({
        "id": "p1",
        "status": "Open",
        "start_date": "2024-01-01",
        "end_date": "2024-01-31"
    });
    ds.insert("accounting_period", period).await.unwrap();

    let plugin = FinancePlugin;
    let schema_arc = Arc::new(schema);
    let entity_data = json!({
        "date": "2024-01-15"
    });

    let args = vec![Expression::StringLiteral("AccountingPeriod".to_string())];

    let result = plugin
        .check_precondition(
            "period_open",
            &args,
            &HashMap::new(),
            &entity_data,
            &schema_arc,
            Some(&ds),
        )
        .await;

    assert!(result.is_ok(), "Check failed with MemoryDataStore: {:?}", result.err());
}
