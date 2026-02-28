use gurih_ir::{EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::plugins::Plugin;
use gurih_runtime::store::{DataStore, MemoryDataStore};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_finance_memory_store_compatibility() {
    // 1. Setup Schema (No database config -> MemoryDataStore)
    let mut schema = Schema {
        database: None,
        ..Default::default()
    };

    // Helper to create basic field
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

    // Account Entity
    schema.entities.insert(
        Symbol::from("Account"),
        EntitySchema {
            name: Symbol::from("Account"),
            table_name: Symbol::from("account"),
            fields: vec![
                field("id", FieldType::Pk),
                field("code", FieldType::String),
                field("name", FieldType::String),
                field("requires_party", FieldType::Boolean),
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // Customer Entity
    schema.entities.insert(
        Symbol::from("Customer"),
        EntitySchema {
            name: Symbol::from("Customer"),
            table_name: Symbol::from("customer"),
            fields: vec![field("id", FieldType::Pk), field("name", FieldType::String)],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // JournalLine Entity
    schema.entities.insert(
        Symbol::from("JournalLine"),
        EntitySchema {
            name: Symbol::from("JournalLine"),
            table_name: Symbol::from("journal_line"),
            fields: vec![
                field("id", FieldType::Pk),
                field("journal_entry", FieldType::Uuid),
                field("account", FieldType::Uuid),
                field("party_type", FieldType::String),
                field("party_id", FieldType::Uuid),
                field("debit", FieldType::Money),
                field("credit", FieldType::Money),
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    let schema_arc = Arc::new(schema);
    // Explicitly use MemoryDataStore
    let datastore = Arc::new(MemoryDataStore::new());
    let ds: Arc<dyn DataStore> = datastore.clone();

    // 2. Seed Data
    let account = json!({
        "id": "acc-1",
        "code": "ACC-1",
        "name": "Account 1",
        "requires_party": true
    });
    // Use table name "account"
    ds.insert("account", account).await.expect("Failed to insert account");

    let customer = json!({
        "id": "cust-1",
        "name": "Customer 1"
    });
    // Use table name "customer"
    ds.insert("customer", customer)
        .await
        .expect("Failed to insert customer");

    let journal_id = Uuid::new_v4().to_string();
    let line = json!({
        "id": "line-1",
        "journal_entry": journal_id,
        "account": "acc-1",
        "debit": "100.00",
        "credit": "0.00",
        "party_type": "Customer",
        "party_id": "cust-1"
    });
    // Use table name "journal_line"
    ds.insert("journal_line", line).await.expect("Failed to insert line");

    let entity_data = json!({
        "id": journal_id,
    });

    // 3. Test Check
    let plugin = FinancePlugin;
    let result = plugin
        .check_precondition(
            "valid_parties",
            &[],
            &HashMap::new(),
            &entity_data,
            &schema_arc,
            Some(&ds),
        )
        .await;

    assert!(result.is_ok(), "Check failed with MemoryDataStore: {:?}", result.err());
}
