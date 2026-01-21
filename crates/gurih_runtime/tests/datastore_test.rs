use gurih_runtime::datastore::{DataStore, DatabaseDataStore};
use gurih_runtime::store::DbPool;
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use std::time::Instant;

#[tokio::test]
async fn bench_list_large_dataset() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1) // Force 1 connection to avoid shared cache issues for simple test
        .connect("sqlite::memory:")
        .await
        .unwrap();

    // Create a table
    sqlx::query("CREATE TABLE test_entity (id TEXT PRIMARY KEY, name TEXT, value INTEGER)")
        .execute(&pool)
        .await
        .unwrap();

    let db_pool = DbPool::Sqlite(pool);
    let datastore = DatabaseDataStore::new(db_pool);
    let datastore_arc = Arc::new(datastore);

    // Insert 10,000 records
    let n = 10000;
    println!("Inserting {} records...", n);
    for i in 0..n {
        datastore_arc
            .insert(
                "test_entity",
                json!({
                    "id": format!("id-{}", i),
                    "name": format!("Item {}", i),
                    "value": i
                }),
            )
            .await
            .unwrap();
    }

    // Measure list all
    let start = Instant::now();
    let items = datastore_arc.list("test_entity", None, None).await.unwrap();
    let duration = start.elapsed();

    println!("List all returned {} items in {:?}", items.len(), duration);

    assert_eq!(items.len(), n);

    // Measure list with limit
    let start_limit = Instant::now();
    let limit_items = datastore_arc.list("test_entity", Some(10), Some(5)).await.unwrap();
    let duration_limit = start_limit.elapsed();

    println!(
        "List limit returned {} items in {:?}",
        limit_items.len(),
        duration_limit
    );

    assert_eq!(limit_items.len(), 10);
    // Verify it is faster (it should be drastically faster if the dataset is large enough)
    // But for 10k items in memory sqlite, the difference might be small (parsing overhead vs data copy).
    // But logically it is optimized.
}
