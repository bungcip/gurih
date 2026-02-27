use gurih_ir::{
    ColumnSchema, ColumnType, DatabaseSchema, DatabaseType, EntitySchema, FieldSchema, FieldType, HierarchySchema,
    QuerySchema, QueryType, Schema, Symbol, TableSchema,
};
use gurih_runtime::data::DataEngine;
use gurih_runtime::store::init_datastore;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_hierarchy_query_rollup() {
    let mut schema = Schema {
        database: Some(DatabaseSchema {
            db_type: DatabaseType::Sqlite,
            url: "sqlite::memory:".to_string(),
        }),
        ..Default::default()
    };

    // 1. Define Entity "Account"
    schema.entities.insert(
        Symbol::from("Account"),
        EntitySchema {
            name: Symbol::from("Account"),
            table_name: Symbol::from("account"),
            fields: vec![
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
                    references: Some(Symbol::from("Account")),
                    serial_generator: None,
                    storage: None,
                    resize: None,
                    filetype: None,
                },
                FieldSchema {
                    name: Symbol::from("balance"),
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
            ],
            relationships: vec![],
            options: HashMap::new(),
            seeds: None,
        },
    );

    // 1.5 Define Table (Required for persistence migration)
    schema.tables.insert(
        Symbol::from("account"),
        TableSchema {
            name: Symbol::from("account"),
            columns: vec![
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
                    name: Symbol::from("balance"),
                    type_name: ColumnType::Float,
                    props: HashMap::new(),
                    primary: false,
                    unique: false,
                },
            ],
        },
    );

    // 2. Define Query "HierarchyQuery"
    schema.queries.insert(
        Symbol::from("HierarchyQuery"),
        QuerySchema {
            name: Symbol::from("HierarchyQuery"),
            params: vec![],
            root_entity: Symbol::from("Account"),
            query_type: QueryType::Hierarchy,
            selections: vec![],
            formulas: vec![],
            filters: vec![],
            joins: vec![],
            group_by: vec![],
            hierarchy: Some(HierarchySchema {
                parent_field: Symbol::from("parent"),
                rollup_fields: vec![Symbol::from("balance")],
            }),
        },
    );

    let schema_arc = Arc::new(schema);
    let datastore = init_datastore(schema_arc.clone(), None).await.unwrap();
    let engine = DataEngine::new(schema_arc, datastore);
    let ctx = gurih_runtime::context::RuntimeContext::system();

    // 3. Seed Data
    // Create Root: Assets (100) - Initial Balance 0
    engine
        .create(
            "Account",
            json!({
                "id": "100",
                "parent": null,
                "balance": "0.0"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // Create Child: Current Assets (101) - Initial Balance 0
    engine
        .create(
            "Account",
            json!({
                "id": "101",
                "parent": "100",
                "balance": "0.0"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // Create Grandchild 1: Cash (101-1) - Balance 500
    engine
        .create(
            "Account",
            json!({
                "id": "101-1",
                "parent": "101",
                "balance": "500.0"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // Create Grandchild 2: Bank (101-2) - Balance 1000
    engine
        .create(
            "Account",
            json!({
                "id": "101-2",
                "parent": "101",
                "balance": "1000.0"
            }),
            &ctx,
        )
        .await
        .unwrap();

    // 4. Execute Query
    let result = engine.list("HierarchyQuery", None, None, None, &ctx).await.unwrap();

    // 5. Verify
    assert_eq!(result.len(), 4);

    // Helper to find by ID since order of siblings is not guaranteed
    let find_by_id = |id: &str| -> serde_json::Map<String, serde_json::Value> {
        result
            .iter()
            .find(|r| r.as_object().unwrap().get("id").unwrap().as_str().unwrap() == id)
            .unwrap()
            .as_object()
            .unwrap()
            .clone()
    };

    let r100 = find_by_id("100");
    assert_eq!(r100.get("balance").unwrap().as_f64().unwrap(), 1500.0);
    assert_eq!(r100.get("_level").unwrap(), 0);
    assert_eq!(r100.get("_has_children").unwrap(), true);

    let r101 = find_by_id("101");
    assert_eq!(r101.get("balance").unwrap().as_f64().unwrap(), 1500.0);
    assert_eq!(r101.get("_level").unwrap(), 1);
    assert_eq!(r101.get("_has_children").unwrap(), true);

    let r101_1 = find_by_id("101-1");
    assert_eq!(r101_1.get("balance").unwrap().as_f64().unwrap(), 500.0);
    assert_eq!(r101_1.get("_level").unwrap(), 2);
    assert_eq!(r101_1.get("_is_leaf").unwrap(), true);

    let r101_2 = find_by_id("101-2");
    assert_eq!(r101_2.get("balance").unwrap().as_f64().unwrap(), 1000.0);
    assert_eq!(r101_2.get("_level").unwrap(), 2);
    assert_eq!(r101_2.get("_is_leaf").unwrap(), true);
}
