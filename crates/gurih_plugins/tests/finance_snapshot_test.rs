use gurih_ir::{EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::datastore::{DataStore, MemoryDataStore};
use gurih_runtime::plugins::Plugin;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_snapshot_parties() {
    // 1. Setup Schema
    let mut schema = Schema::default();
    schema.database = None; // Use Memory

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
                field("party_type", FieldType::String),
                field("party_id", FieldType::Uuid),
                field("party_name", FieldType::String), // The new field
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    let schema_arc = Arc::new(schema);
    let ds: Arc<dyn DataStore> = Arc::new(MemoryDataStore::new());

    // 2. Seed Data
    let customer_id = "cust-123";
    let customer_name = "Acme Corp";
    ds.insert(
        "customer",
        json!({
            "id": customer_id,
            "name": customer_name
        }),
    )
    .await
    .unwrap();

    let journal_id = "je-001";
    let line_id = "line-001";

    // Create Line without party_name
    ds.insert(
        "journal_line",
        json!({
            "id": line_id,
            "journal_entry": journal_id,
            "party_type": "Customer",
            "party_id": customer_id
        }),
    )
    .await
    .unwrap();

    // 3. Execute Plugin Effect
    let plugin = FinancePlugin;
    let entity_data = json!({ "id": journal_id });

    let result = plugin
        .apply_effect(
            "snapshot_parties",
            &[],
            &HashMap::new(),
            &schema_arc,
            Some(&ds),
            "JournalEntry",
            &entity_data,
        )
        .await;

    assert!(result.is_ok(), "Plugin execution failed: {:?}", result.err());

    // 4. Verify Snapshot
    let line_after = ds.get("journal_line", line_id).await.unwrap().unwrap();
    let party_name = line_after.get("party_name").and_then(|v| v.as_str());

    assert_eq!(party_name, Some(customer_name));

    // 5. Test Audit (Change customer name, snapshot shouldn't change if already set)
    // Update Customer
    ds.update("customer", customer_id, json!({ "name": "Acme Inc" }))
        .await
        .unwrap();

    // Execute again
    let result2 = plugin
        .apply_effect(
            "snapshot_parties",
            &[],
            &HashMap::new(),
            &schema_arc,
            Some(&ds),
            "JournalEntry",
            &entity_data,
        )
        .await;
    assert!(result2.is_ok());

    // Verify it did NOT change (because logic checks if missing/empty)
    let line_after_2 = ds.get("journal_line", line_id).await.unwrap().unwrap();
    let party_name_2 = line_after_2.get("party_name").and_then(|v| v.as_str());

    assert_eq!(party_name_2, Some(customer_name));
}
