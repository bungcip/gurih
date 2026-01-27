use gurih_ir::{DatabaseType, EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_runtime::persistence::SchemaManager;
use gurih_runtime::store::DbPool;
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_seed_sorting() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    let mut seeds = HashMap::new();
    seeds.insert("id".to_string(), "seed-1".to_string());
    seeds.insert("name".to_string(), "Seed 1".to_string());
    seeds.insert("description".to_string(), "Description 1".to_string());
    seeds.insert("active".to_string(), "true".to_string());
    seeds.insert("count".to_string(), "10".to_string());

    let fields = vec![
        FieldSchema {
            name: Symbol::from("id"),
            field_type: FieldType::String,
            required: true,
            unique: true,
            default: None,
            references: None,
            serial_generator: None,
            storage: None,
            resize: None,
            filetype: None,
        },
        FieldSchema {
            name: Symbol::from("name"),
            field_type: FieldType::String,
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
            name: Symbol::from("description"),
            field_type: FieldType::String,
            required: false,
            unique: false,
            default: None,
            references: None,
            serial_generator: None,
            storage: None,
            resize: None,
            filetype: None,
        },
        FieldSchema {
            name: Symbol::from("active"),
            field_type: FieldType::Boolean,
            required: false,
            unique: false,
            default: None,
            references: None,
            serial_generator: None,
            storage: None,
            resize: None,
            filetype: None,
        },
        FieldSchema {
            name: Symbol::from("count"),
            field_type: FieldType::Integer,
            required: false,
            unique: false,
            default: None,
            references: None,
            serial_generator: None,
            storage: None,
            resize: None,
            filetype: None,
        },
    ];

    let entity = EntitySchema {
        name: Symbol::from("TestEntity"),
        fields,
        relationships: vec![],
        options: HashMap::new(),
        seeds: Some(vec![seeds]),
    };

    let mut entities = HashMap::new();
    entities.insert(Symbol::from("TestEntity"), entity);

    let schema = Schema {
        entities,
        ..Default::default()
    };

    let schema_arc = Arc::new(schema);
    let manager = SchemaManager::new(DbPool::Sqlite(pool.clone()), schema_arc, DatabaseType::Sqlite);

    manager.migrate().await.unwrap();

    // Query to verify insertion
    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM TestEntity")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(count.0, 1);
}
