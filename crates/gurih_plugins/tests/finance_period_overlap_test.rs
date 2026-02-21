use gurih_ir::{EntitySchema, Schema, Symbol};
use gurih_plugins::finance::FinancePlugin;
use gurih_runtime::datastore::{DataStore, MemoryDataStore};
use gurih_runtime::plugins::Plugin;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_period_overlap_check() {
    let datastore = Arc::new(MemoryDataStore::new());

    // 1. Setup existing periods
    // Period 1: Jan 2024 (Open)
    let p1 = json!({
        "id": "p1",
        "name": "Jan 2024",
        "start_date": "2024-01-01",
        "end_date": "2024-01-31",
        "status": "Open"
    });
    datastore.insert("accounting_period", p1).await.unwrap();

    // Period 2: Feb 2024 (Draft) - should be ignored
    let p2 = json!({
        "id": "p2",
        "name": "Feb 2024",
        "start_date": "2024-02-01",
        "end_date": "2024-02-29",
        "status": "Draft"
    });
    datastore.insert("accounting_period", p2).await.unwrap();

    // Setup Schema
    let mut schema = Schema::default();
    let entity_name = Symbol::from("AccountingPeriod");
    schema.entities.insert(
        entity_name.clone(),
        EntitySchema {
            name: entity_name,
            table_name: Symbol::from("accounting_period"),
            fields: vec![],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    let plugin = FinancePlugin;
    let kwargs = HashMap::new();

    // Test Case 1: Create new period overlapping with p1 (Open)
    // Overlap: Jan 15 - Feb 15
    let new_period_overlap = json!({
        "id": "new1",
        "name": "Mid-Jan to Mid-Feb",
        "start_date": "2024-01-15",
        "end_date": "2024-02-15",
        "status": "Draft" // checking precondition for activation, but passed data is usually what's in DB or proposed
    });

    // check_precondition doesn't care about status of the object being checked,
    // it checks if *other* periods overlap with its dates.

    let res = plugin
        .check_precondition(
            "no_period_overlap",
            &[],
            &kwargs,
            &new_period_overlap,
            &schema,
            Some(&(datastore.clone() as Arc<dyn DataStore>)),
        )
        .await;

    assert!(res.is_err(), "Should fail due to overlap with Jan 2024");
    let err = res.err().unwrap().to_string();
    assert!(err.contains("overlaps with existing period 'Jan 2024'"));

    // Test Case 2: Overlap with Draft period (p2) -> Should Pass
    // Overlap: Feb 10 - Feb 20
    let new_period_overlap_draft = json!({
        "id": "new2",
        "name": "Feb overlap",
        "start_date": "2024-02-10",
        "end_date": "2024-02-20"
    });

    let res = plugin
        .check_precondition(
            "no_period_overlap",
            &[],
            &kwargs,
            &new_period_overlap_draft,
            &schema,
            Some(&(datastore.clone() as Arc<dyn DataStore>)),
        )
        .await;

    assert!(
        res.is_ok(),
        "Should pass because overlap is only with Draft period (Feb 2024)"
    );

    // Test Case 3: No overlap (March 2024)
    let new_period_ok = json!({
        "id": "new3",
        "name": "Mar 2024",
        "start_date": "2024-03-01",
        "end_date": "2024-03-31"
    });

    let res = plugin
        .check_precondition(
            "no_period_overlap",
            &[],
            &kwargs,
            &new_period_ok,
            &schema,
            Some(&(datastore.clone() as Arc<dyn DataStore>)),
        )
        .await;

    assert!(res.is_ok(), "Should pass (no overlap)");

    // Test Case 4: Invalid Dates (Start > End)
    let invalid_dates = json!({
        "id": "inv1",
        "start_date": "2024-05-01",
        "end_date": "2024-04-01"
    });

    let res = plugin
        .check_precondition(
            "no_period_overlap",
            &[],
            &kwargs,
            &invalid_dates,
            &schema,
            Some(&(datastore.clone() as Arc<dyn DataStore>)),
        )
        .await;

    assert!(res.is_err());
    assert!(res.err().unwrap().to_string().contains("Start date must be before"));
}
