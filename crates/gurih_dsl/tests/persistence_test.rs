use gurih_dsl::compile;
use gurih_ir::{ColumnType, Symbol};

#[test]
fn test_compile_table_and_database() {
    let src = r#"
    database {
        type "postgres"
        url "env:DATABASE_URL"
    }

    entity "Product" {
        pk "id"
        field "name" type="String"
    }
    "#;

    let schema = compile(src, None).expect("Should compile");

    // Check Database
    let db = schema.database.expect("Database should be present");
    assert_eq!(db.db_type, gurih_ir::DatabaseType::Postgres);
    assert_eq!(db.url, "env:DATABASE_URL");

    // Check Table (generated from Entity)
    assert!(schema.tables.contains_key(&Symbol::from("product")));
}
