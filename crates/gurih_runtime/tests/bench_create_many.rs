use gurih_ir::{EntitySchema, FieldSchema, FieldType, RuleSchema, Symbol, Schema};
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::MemoryDataStore;
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;

#[tokio::test]
async fn bench_create_many_performance() {
    let mut schema = Schema::default();
    let entity_name = Symbol::from("BenchItem");

    let entity = EntitySchema {
        name: entity_name,
        table_name: Symbol::from("bench_items"),
        fields: vec![
            FieldSchema {
                name: Symbol::from("name"),
                field_type: FieldType::Text,
                required: true,
                unique: false,
                default: None,
                references: None,
                serial_generator: None,
                storage: None,
                resize: None,
                filetype: None,
            },
            FieldSchema {
                name: Symbol::from("value"),
                field_type: FieldType::Integer,
                required: true,
                unique: false,
                default: None,
                references: None,
                serial_generator: None,
                storage: None,
                resize: None,
                filetype: None,
            },
        ],
        relationships: vec![],
        options: Default::default(),
        seeds: None,
    };
    schema.entities.insert(entity_name, entity);

    // Add a rule to make check_rules do some work
    // A simple rule that always passes but requires evaluation
    let rule = RuleSchema {
        name: Symbol::from("AlwaysTrue"),
        on_event: Symbol::from("BenchItem:create"),
        assertion: gurih_ir::Expression::BinaryOp {
            left: Box::new(gurih_ir::Expression::Literal(1.0)),
            op: gurih_ir::BinaryOperator::Eq,
            right: Box::new(gurih_ir::Expression::Literal(1.0)),
        },
        message: "Should not fail".to_string(),
    };
    schema.rules.insert(rule.name, rule);

    let datastore = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(Arc::new(schema), datastore);

    let ctx = RuntimeContext {
        user_id: "bench_user".to_string(),
        roles: vec!["admin".to_string()],
        permissions: vec!["create:BenchItem".to_string()],
        token: None,
    };

    let count = 2000;
    let mut data = Vec::with_capacity(count);
    for i in 0..count {
        data.push(json!({
            "name": format!("Item {}", i),
            "value": i
        }));
    }

    println!("Starting create_many benchmark with {} items...", count);
    let start = Instant::now();

    let result = engine.create_many("BenchItem", data, &ctx).await;

    let duration = start.elapsed();

    match result {
        Ok(ids) => {
            assert_eq!(ids.len(), count);
            println!("BENCH_RESULT: create_many {} items took: {:?}", count, duration);
        }
        Err(e) => {
            panic!("create_many failed: {}", e);
        }
    }
}
