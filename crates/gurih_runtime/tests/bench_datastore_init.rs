use gurih_ir::{DatabaseType, Schema, DatabaseSchema};
use gurih_runtime::datastore::init_datastore;
use std::sync::Arc;
use std::time::Instant;
use tokio::fs;

#[tokio::test]
async fn bench_init_datastore() {
    let tmp_dir = std::env::temp_dir().join("bench_datastore_init");
    if tmp_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&tmp_dir).await {
             println!("Failed to remove dir: {}", e);
             // Maybe it doesn't exist?
        }
    }

    // We want to test if init_datastore creates the directory.
    // The path will be tmp_dir/test.db
    let db_path = tmp_dir.join("test.db");

    // We make sure tmp_dir does NOT exist
    if tmp_dir.exists() {
         fs::remove_dir_all(&tmp_dir).await.unwrap();
    }

    let url = format!("sqlite://{}", db_path.display());

    let schema = Arc::new(Schema {
        database: Some(DatabaseSchema {
            url: url.clone(),
            db_type: DatabaseType::Sqlite,
        }),
        ..Default::default()
    });

    println!("Initializing datastore at {}", url);
    let start = Instant::now();
    let result = init_datastore(schema.clone(), None).await;
    let duration = start.elapsed();

    if let Err(e) = &result {
        println!("Error: {}", e);
    }
    assert!(result.is_ok(), "init_datastore failed");

    println!("init_datastore took: {:?}", duration);

    // Verify file exists
    assert!(db_path.exists(), "DB file was not created");

    // Cleanup
    if tmp_dir.exists() {
        fs::remove_dir_all(&tmp_dir).await.unwrap();
    }
}
