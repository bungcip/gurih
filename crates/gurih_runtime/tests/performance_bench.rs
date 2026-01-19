use gurih_runtime::storage::{MemoryStorage, Storage};
use serde_json::json;
use std::time::Instant;

#[tokio::test]
async fn bench_list_performance() {
    let storage = MemoryStorage::new();
    let entity = "bench_entity";
    let count = 50000; // Large enough to notice difference

    println!("Seeding {} items...", count);
    for i in 0..count {
        let record = json!({
            "name": format!("Item {}", i),
            "value": i,
            "description": "Some long description text to make the payload heavier. ".repeat(10),
            "meta": {
                "tags": ["a", "b", "c"],
                "active": true
            }
        });
        storage.insert(entity, record).await.unwrap();
    }
    println!("Seeding complete.");

    let start = Instant::now();
    let items = storage.list(entity).await.unwrap();
    let duration = start.elapsed();

    println!("BENCH_RESULT: List {} items took: {:?}", items.len(), duration);
    assert_eq!(items.len(), count);
}
