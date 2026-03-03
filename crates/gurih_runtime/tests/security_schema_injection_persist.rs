use gurih_ir::{ColumnSchema, ColumnType, DatabaseType, Schema, Symbol, TableSchema};
use gurih_runtime::persistence::SchemaManager;
use gurih_runtime::store::DbPool;
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_sql_injection_in_schema_table_name() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();

    let malicious_table_name = "test_table\"; CREATE TABLE hacked (id TEXT); --";

    let mut tables = HashMap::new();
    tables.insert(
        Symbol::from("malicious"),
        TableSchema {
            name: Symbol::from(malicious_table_name),
            columns: vec![
                ColumnSchema {
                    name: Symbol::from("id"),
                    type_name: ColumnType::Text,
                    props: HashMap::new(),
                    primary: true,
                    unique: true,
                },
            ],
        },
    );

    let schema = Schema {
        tables,
        ..Default::default()
    };

    let schema_arc = Arc::new(schema);
    let manager = SchemaManager::new(DbPool::Sqlite(pool.clone()), schema_arc, DatabaseType::Sqlite);

    let result = manager.migrate().await;

    // With validate_identifier, this should immediately fail with "Invalid identifier"
    assert!(result.is_err(), "Migration should fail on invalid table name");
    assert!(result.unwrap_err().contains("Invalid identifier"), "Should fail due to identifier validation");
}
