use gurih_runtime::datastore::init_datastore;
use gurih_dsl::compile;
use gurih_runtime::data::DataEngine;
use gurih_runtime::context::RuntimeContext;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_audit_trail() {
    let dsl = r#"
    database {
        type "sqlite"
        url "sqlite::memory:"
    }
    entity "TestEntity" {
        field:pk id
        field:string "name"
        options {
            track_changes #true
        }
    }
    entity "NoTrackEntity" {
        field:pk id
        field:string "name"
        // default track_changes is false
    }
    "#;

    let schema = compile(dsl, None).expect("DSL compilation failed");
    let schema = Arc::new(schema);

    // Initialize Datastore (this runs migrate -> create_audit_table)
    let datastore = init_datastore(schema.clone(), None).await.expect("Datastore init failed");

    let engine = DataEngine::new(schema.clone(), datastore.clone());
    let ctx = RuntimeContext::system();

    // 1. Create TestEntity (Tracked)
    let id = engine.create("TestEntity", json!({
        "name": "Audit Me"
    }), &ctx).await.expect("Create failed");

    // Verify Audit Log
    // SqliteDataStore::query returns Vec<Arc<Value>>
    let logs = datastore.query("SELECT * FROM \"_audit_log\" WHERE entity = 'TestEntity'").await.expect("Query failed");
    assert_eq!(logs.len(), 1, "Should have 1 audit log for CREATE");
    let log = &logs[0];
    assert_eq!(log.get("action").unwrap().as_str().unwrap(), "CREATE");
    assert_eq!(log.get("record_id").unwrap().as_str().unwrap(), id);

    // 2. Update TestEntity
    engine.update("TestEntity", &id, json!({
        "name": "Audit Me Updated"
    }), &ctx).await.expect("Update failed");

    let logs = datastore.query("SELECT * FROM \"_audit_log\" WHERE entity = 'TestEntity' ORDER BY timestamp ASC").await.expect("Query failed");
    assert_eq!(logs.len(), 2, "Should have 2 audit logs");

    // Sort logic is handled by SQL.
    let log_update = &logs[1];
    assert_eq!(log_update.get("action").unwrap().as_str().unwrap(), "UPDATE");

    let diff_str = log_update.get("diff").unwrap().as_str().unwrap();
    let diff: serde_json::Value = serde_json::from_str(diff_str).unwrap();
    assert!(diff.get("name").is_some());
    assert_eq!(diff["name"]["old"], "Audit Me");
    assert_eq!(diff["name"]["new"], "Audit Me Updated");

    // 3. Create NoTrackEntity
    engine.create("NoTrackEntity", json!({
        "name": "Ignore Me"
    }), &ctx).await.expect("Create failed");

    let logs = datastore.query("SELECT * FROM \"_audit_log\" WHERE entity = 'NoTrackEntity'").await.expect("Query failed");
    assert_eq!(logs.len(), 0, "Should have 0 audit logs for untracked entity");

    // 4. Delete TestEntity
    engine.delete("TestEntity", &id, &ctx).await.expect("Delete failed");

    let logs = datastore.query("SELECT * FROM \"_audit_log\" WHERE entity = 'TestEntity' ORDER BY timestamp ASC").await.expect("Query failed");
    assert_eq!(logs.len(), 3, "Should have 3 audit logs (Create, Update, Delete)");
    let log_delete = &logs[2];
    assert_eq!(log_delete.get("action").unwrap().as_str().unwrap(), "DELETE");
    assert_eq!(log_delete.get("record_id").unwrap().as_str().unwrap(), id);
}
