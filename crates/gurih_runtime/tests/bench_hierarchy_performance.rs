use gurih_ir::{
    ColumnSchema, ColumnType, DatabaseSchema, DatabaseType, EntitySchema, FieldSchema, FieldType, HierarchySchema,
    QuerySchema, QueryType, Schema, Symbol, TableSchema,
};
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::init_datastore;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

#[tokio::test]
async fn test_hierarchy_performance() {
    let mut schema = Schema {
        database: Some(DatabaseSchema {
            db_type: DatabaseType::Sqlite,
            url: "sqlite::memory:".to_string(),
        }),
        ..Default::default()
    };

    // 1. Define Entity "Node" with many fields
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
            name: Symbol::from("parent"),
            field_type: FieldType::Relation,
            required: false,
            unique: false,
            default: None,
            references: Some(Symbol::from("Node")),
            serial_generator: None,
            storage: None,
            resize: None,
            filetype: None,
        },
        FieldSchema {
            name: Symbol::from("value"),
            field_type: FieldType::Money,
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

    let mut columns = vec![
        ColumnSchema {
            name: Symbol::from("id"),
            type_name: ColumnType::Text,
            props: HashMap::new(),
            primary: true,
            unique: true,
        },
        ColumnSchema {
            name: Symbol::from("parent"),
            type_name: ColumnType::Text,
            props: HashMap::new(),
            primary: false,
            unique: false,
        },
        ColumnSchema {
            name: Symbol::from("value"),
            type_name: ColumnType::Float,
            props: HashMap::new(),
            primary: false,
            unique: false,
        },
    ];

    // Add 50 dummy fields to simulate heavy record
    for i in 1..=50 {
        let name = format!("col_{}", i);
        fields.push(FieldSchema {
            name: Symbol::from(&name),
            field_type: FieldType::Text,
            required: false,
            unique: false,
            default: None,
            references: None,
            serial_generator: None,
            storage: None,
            resize: None,
            filetype: None,
        });
        columns.push(ColumnSchema {
            name: Symbol::from(&name),
            type_name: ColumnType::Text,
            props: HashMap::new(),
            primary: false,
            unique: false,
        });
    }

    schema.entities.insert(
        Symbol::from("Node"),
        EntitySchema {
            name: Symbol::from("Node"),
            table_name: Symbol::from("node"),
            fields,
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    schema.tables.insert(
        Symbol::from("node"),
        TableSchema {
            name: Symbol::from("node"),
            columns,
        },
    );

    schema.queries.insert(
        Symbol::from("HierarchyQuery"),
        QuerySchema {
            name: Symbol::from("HierarchyQuery"),
            params: vec![],
            root_entity: Symbol::from("Node"),
            query_type: QueryType::Hierarchy,
            selections: vec![],
            formulas: vec![],
            filters: vec![],
            joins: vec![],
            group_by: vec![],
            hierarchy: Some(HierarchySchema {
                parent_field: Symbol::from("parent"),
                rollup_fields: vec![Symbol::from("value")],
            }),
        },
    );

    let schema_arc = Arc::new(schema);
    let datastore = init_datastore(schema_arc.clone(), None).await.unwrap();
    let engine = DataEngine::new(schema_arc, datastore);
    let ctx = gurih_runtime::context::RuntimeContext::system();

    // 3. Seed Data (2000 nodes)
    println!("Seeding data...");
    let mut records = Vec::new();

    // Create large filler text
    let filler = "x".repeat(100);

    // Root
    let mut root_obj = json!({
        "id": "root",
        "parent": null,
        "value": "1.0"
    });
    for i in 1..=50 {
        root_obj
            .as_object_mut()
            .unwrap()
            .insert(format!("col_{}", i), json!(filler));
    }
    records.push(root_obj);

    let mut prev_id = "root".to_string();
    for i in 1..2000 {
        let id = format!("node_{}", i);
        let parent = if i % 10 == 0 {
            "root".to_string()
        } else {
            prev_id.clone()
        };

        let mut obj = json!({
            "id": id.clone(),
            "parent": parent,
            "value": "1.0"
        });
        for j in 1..=50 {
            obj.as_object_mut().unwrap().insert(format!("col_{}", j), json!(filler));
        }
        records.push(obj);

        prev_id = id;
    }

    engine.create_many("Node", records, &ctx).await.unwrap();
    println!("Seeding complete.");

    // 4. Benchmark
    let start = Instant::now();
    let result = engine.list("HierarchyQuery", Some(20), None, None, &ctx).await.unwrap();
    let duration = start.elapsed();

    println!("Query took: {:?}", duration);
    println!("Result count: {}", result.len());

    assert_eq!(result.len(), 20); // Limit works

    let root_node = result
        .iter()
        .find(|r| r.as_object().unwrap().get("id").unwrap().as_str().unwrap() == "root")
        .unwrap();
    let root_val = root_node.as_object().unwrap().get("value").unwrap().as_f64().unwrap();
    println!("Root value: {}", root_val);
}
