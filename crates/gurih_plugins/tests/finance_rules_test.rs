use gurih_dsl::compile;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::store::MemoryDataStore;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

#[tokio::test]
async fn test_leaf_account_validation() {
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
    let ctx = RuntimeContext::system(); // Admin

    // 3. Create Group Account
    let group_acc = json!({
        "code": "100",
        "name": "Assets",
        "type": "Asset",
        "normal_balance": "Debit",
        "is_group": true,
        "is_active": true
    });
    let group_id = engine
        .create("Account", group_acc, &ctx)
        .await
        .expect("Failed to create group account");

    // Verify Group Account
    let saved_group = engine.read("Account", &group_id, &ctx).await.unwrap().unwrap();
    println!("Saved Group: {:?}", saved_group);
    assert_eq!(saved_group.get("is_group"), Some(&json!(true)));

    // 4. Create Leaf Account
    let leaf_acc = json!({
        "code": "101",
        "name": "Cash",
        "type": "Asset",
        "normal_balance": "Debit",
        "is_group": false,
        "is_active": true,
        "parent": group_id
    });
    let leaf_id = engine
        .create("Account", leaf_acc, &ctx)
        .await
        .expect("Failed to create leaf account");

    // 5. Create Journal Header
    let journal = json!({
        "description": "Test Journal",
        "date": "2024-01-01",
        "status": "Draft"
    });
    let journal_id = engine
        .create("JournalEntry", journal, &ctx)
        .await
        .expect("Failed to create journal");

    // 6. Try Posting to Group Account (Should Fail)
    let bad_line = json!({
        "journal_entry": journal_id,
        "account": group_id,
        "debit": "100.00",
        "credit": "0.00"
    });
    let res = engine.create("JournalLine", bad_line, &ctx).await;
    assert!(res.is_err(), "Should not allow posting to group account");
    if let Err(msg) = res {
        assert!(
            msg.contains("Cannot post to a group account"),
            "Wrong error message: {}",
            msg
        );
    }

    // 7. Try Posting to Leaf Account (Should Succeed)
    let good_line = json!({
        "journal_entry": journal_id,
        "account": leaf_id,
        "debit": "100.00",
        "credit": "0.00"
    });
    let res = engine.create("JournalLine", good_line, &ctx).await;
    assert!(res.is_ok(), "Should allow posting to leaf account: {:?}", res.err());
}
