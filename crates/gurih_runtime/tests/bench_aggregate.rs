use gurih_runtime::datastore::DataStore;
use gurih_runtime::store::MemoryDataStore;
use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

#[tokio::test]
async fn bench_aggregate_performance() {
    let datastore = MemoryDataStore::new();
    let entity = "bench_entity_agg";
    let count = 100_000;

    println!("Seeding {} items...", count);
    for i in 0..count {
        let category = if i % 2 == 0 { "A" } else { "B" };
        let record = json!({
            "name": format!("Item {}", i),
            "category": category,
            "active": true,
            "value": i,
            "status": if i % 3 == 0 { "Open" } else { "Closed" }
        });
        datastore.insert(entity, record).await.unwrap();
    }
    println!("Seeding complete.");

    // Benchmark aggregate
    let mut filters = HashMap::new();
    filters.insert("category".to_string(), "A".to_string());

    let start = Instant::now();
    // Group by 'status' with filter category='A'
    let result = datastore.aggregate(entity, "status", filters).await.unwrap();
    let duration = start.elapsed();
    println!("BENCH_RESULT: Aggregate (groups {}) took: {:?}", result.len(), duration);

    // Verify logic
    let mut total_count = 0;
    for (_, count) in result {
        total_count += count;
    }
    assert_eq!(total_count, count as i64 / 2);

    // Verify NUMERIC filter (Correctness check)
    println!("Verifying numeric filter correctness...");
    let mut num_filters = HashMap::new();
    // value is unique, so this should match exactly 1 record
    num_filters.insert("value".to_string(), "10".to_string());
    let result_num = datastore.aggregate(entity, "status", num_filters).await.unwrap();

    // In the old implementation, this would likely fail or return empty because "10" (string) != 10 (number) in JSON without specialized logic
    // But MemoryDataStore::matches_filter (old) did `n.to_string() == v`.
    // Wait, the OLD `aggregate` implementation did:
    // record.get(k).and_then(|val| val.as_str())
    // which returns None for numbers! So the old aggregate would FAIL to match numbers entirely.

    println!("Numeric aggregate result: {:?}", result_num);
    assert_eq!(result_num.len(), 1);
    assert_eq!(result_num[0].1, 1);
}
