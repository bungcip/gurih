use gurih_runtime::datastore::{DataStore, MemoryDataStore};
use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

#[tokio::test]
async fn bench_filter_performance() {
    let datastore = MemoryDataStore::new();
    let entity = "bench_entity";
    let count = 100_000;

    println!("Seeding {} items...", count);
    for i in 0..count {
        let category = if i % 2 == 0 { "A" } else { "B" };
        let record = json!({
            "name": format!("Item {}", i),
            "category": category,
            "active": true,
            "value": i
        });
        datastore.insert(entity, record).await.unwrap();
    }
    println!("Seeding complete.");

    // Benchmark count
    let mut filters = HashMap::new();
    filters.insert("category".to_string(), "A".to_string());

    let start = Instant::now();
    let result_count = datastore.count(entity, filters.clone()).await.unwrap();
    let duration = start.elapsed();
    println!("BENCH_RESULT: Count (matches {}) took: {:?}", result_count, duration);

    // Benchmark find
    let start = Instant::now();
    let result_items = datastore.find(entity, filters).await.unwrap();
    let duration = start.elapsed();
    println!(
        "BENCH_RESULT: Find (matches {}) took: {:?}",
        result_items.len(),
        duration
    );

    assert_eq!(result_count, count as i64 / 2);
    assert_eq!(result_items.len(), count as usize / 2);
}
