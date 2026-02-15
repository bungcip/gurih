use gurih_ir::{
    DatabaseSchema, DatabaseType, EntitySchema, FieldSchema, FieldType, Schema, SerialGeneratorSchema, Symbol,
};
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::init_datastore;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[tokio::test]
async fn test_serial_generation() {
    // 1. Define Serial Generator
    let mut serial_generators = HashMap::new();
    let gen_name = Symbol::from("TestGen");

    serial_generators.insert(
        gen_name,
        SerialGeneratorSchema {
            name: gen_name,
            prefix: Some("TEST/".to_string()),
            date_format: Some("%Y/".to_string()),
            digits: 4,
        },
    );

    // 2. Define Entity
    let mut fields = vec![
        FieldSchema {
            name: Symbol::from("id"),
            field_type: FieldType::Pk,
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
            name: Symbol::from("serial_no"),
            field_type: FieldType::Serial,
            required: false,
            unique: true,
            default: None,
            references: None,
            serial_generator: Some(gen_name),
            storage: None,
            resize: None,
            filetype: None,
        },
        FieldSchema {
            name: Symbol::from("title"),
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
    ];

    let entity_name = Symbol::from("Doc");
    let table_name = Symbol::from("doc");

    let entity = EntitySchema {
        name: entity_name,
        table_name,
        fields,
        relationships: vec![],
        options: HashMap::new(),
        seeds: None,
    };

    let mut entities = HashMap::new();
    entities.insert(entity_name, entity);

    // 3. Define Schema with Database
    // Using in-memory sqlite
    let schema = Schema {
        entities,
        serial_generators,
        database: Some(DatabaseSchema {
            db_type: DatabaseType::Sqlite,
            url: "sqlite::memory:".to_string(),
        }),
        ..Default::default()
    };

    let schema_arc = Arc::new(schema);

    // 4. Init Datastore
    // Pass None as base_path for in-memory
    let datastore = init_datastore(schema_arc.clone(), None)
        .await
        .expect("Failed to init datastore");

    // 5. Create DataEngine
    let engine = DataEngine::new(schema_arc.clone(), datastore);
    let ctx = RuntimeContext::system();

    // 6. Create First Doc
    let mut data1 = serde_json::Map::new();
    data1.insert("title".to_string(), Value::String("Doc 1".to_string()));

    // Note: serial_no is not provided, so it should be generated.
    let id1 = engine
        .create("Doc", Value::Object(data1), &ctx)
        .await
        .expect("Failed to create Doc 1");

    // 7. Verify First Serial
    let record1 = engine
        .read("Doc", &id1, &ctx)
        .await
        .expect("Read failed")
        .expect("Doc 1 not found");
    let serial1 = record1
        .get("serial_no")
        .and_then(|v| v.as_str())
        .expect("serial_no 1 missing");

    let year = chrono::Local::now().format("%Y").to_string();
    let expected1 = format!("TEST/{}/0001", year);

    assert_eq!(serial1, expected1);

    // 8. Create Second Doc
    let mut data2 = serde_json::Map::new();
    data2.insert("title".to_string(), Value::String("Doc 2".to_string()));

    let id2 = engine
        .create("Doc", Value::Object(data2), &ctx)
        .await
        .expect("Failed to create Doc 2");

    // 9. Verify Second Serial
    let record2 = engine
        .read("Doc", &id2, &ctx)
        .await
        .expect("Read failed")
        .expect("Doc 2 not found");
    let serial2 = record2
        .get("serial_no")
        .and_then(|v| v.as_str())
        .expect("serial_no 2 missing");

    let expected2 = format!("TEST/{}/0002", year);
    assert_eq!(serial2, expected2);
}
