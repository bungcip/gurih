use gurih_runtime::datastore::DataStore;
use gurih_runtime::store::sqlite::SqliteDataStore;
use serde_json::Value;
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_database_error_sanitization() {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create pool");

    let store = SqliteDataStore::new(pool);

    // Create table with unique constraint
    store.query("CREATE TABLE users (id TEXT PRIMARY KEY, email TEXT UNIQUE)").await.expect("Failed to create table");

    let user1 = serde_json::json!({
        "id": "1",
        "email": "test@example.com"
    });

    store.insert("users", user1.clone()).await.expect("First insert should succeed");

    // Insert duplicate
    let user2 = serde_json::json!({
        "id": "2",
        "email": "test@example.com"
    });

    let err = store.insert("users", user2).await.expect_err("Duplicate insert should fail");

    let err_msg = err.to_string();
    println!("Received error: {}", err_msg);

    // Assert that we DO NOT expose raw unique constraint error
    assert!(!err_msg.contains("UNIQUE constraint failed"), "Error should be sanitized: {}", err_msg);

    // Assert we return a friendly message
    assert!(err_msg.contains("Duplicate entry"), "Error should contain friendly message: {}", err_msg);
}
