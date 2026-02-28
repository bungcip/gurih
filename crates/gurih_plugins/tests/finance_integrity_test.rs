use gurih_dsl::compile;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::store::MemoryDataStore;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

#[tokio::test]
async fn test_prevent_account_delete_in_use() {
    // 1. Load Schema
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let finance_path = root.join("gurih-finance");
    let gurih_kdl = finance_path.join("gurih.kdl");

    let content = std::fs::read_to_string(&gurih_kdl).expect("Failed to read gurih.kdl");
    let schema = compile(&content, Some(&finance_path)).expect("Failed to compile schema");
    let schema_arc = Arc::new(schema);

    // 2. Setup Engine
    let datastore = Arc::new(MemoryDataStore::new());
    // Explicitly cast to trait object if needed, but DataEngine::new expects Arc<dyn DataStore>
    // MemoryDataStore implements DataStore.
    // The previous error in unit test was because evaluate() expected &Arc<dyn DataStore>.
    // DataEngine::new expects Arc<dyn DataStore>.
    let engine = DataEngine::new(schema_arc.clone(), datastore.clone());
    let ctx = RuntimeContext::system();

    // 3. Create Account
    let acc_data = json!({
        "code": "101",
        "name": "Cash",
        "type": "Asset",
        "normal_balance": "Debit",
        "is_group": false,
        "is_active": true
    });
    let acc_id = engine
        .create("Account", acc_data, &ctx)
        .await
        .expect("Failed to create account");

    // 4. Create Journal Entry
    let journal = json!({
        "description": "Test Journal",
        "date": "2024-01-01",
        "status": "Draft"
    });
    let journal_id = engine
        .create("JournalEntry", journal, &ctx)
        .await
        .expect("Failed to create journal");

    // 5. Create Journal Line (Using Account)
    let line = json!({
        "journal_entry": journal_id,
        "account": acc_id,
        "debit": "100.00",
        "credit": "0.00"
    });
    engine
        .create("JournalLine", line, &ctx)
        .await
        .expect("Failed to create journal line");

    // 6. Try Delete Account (Should Fail)
    let res = engine.delete("Account", &acc_id, &ctx).await;
    assert!(res.is_err(), "Should not allow deleting used account");
    if let Err(msg) = res {
        assert!(
            msg.contains("Cannot delete account that has journal entries"),
            "Wrong error message: {}",
            msg
        );
    }
}

#[tokio::test]
async fn test_allow_account_delete_unused() {
    // 1. Load Schema
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let finance_path = root.join("gurih-finance");
    let gurih_kdl = finance_path.join("gurih.kdl");

    let content = std::fs::read_to_string(&gurih_kdl).expect("Failed to read gurih.kdl");
    let schema = compile(&content, Some(&finance_path)).expect("Failed to compile schema");
    let schema_arc = Arc::new(schema);

    // 2. Setup Engine
    let datastore = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(schema_arc.clone(), datastore.clone());
    let ctx = RuntimeContext::system();

    // 3. Create Account
    let acc_data = json!({
        "code": "102",
        "name": "Bank",
        "type": "Asset",
        "normal_balance": "Debit",
        "is_group": false,
        "is_active": true
    });
    let acc_id = engine
        .create("Account", acc_data, &ctx)
        .await
        .expect("Failed to create account");

    // 4. Try Delete Account (Should Succeed)
    let res = engine.delete("Account", &acc_id, &ctx).await;
    assert!(res.is_ok(), "Should allow deleting unused account");
}
