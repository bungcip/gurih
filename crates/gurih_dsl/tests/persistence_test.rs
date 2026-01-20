use gurih_dsl::compile;

#[test]
fn test_compile_table_and_database() {
    let src = r#"
    database {
        type "postgres"
        url "env:DATABASE_URL"
    }

    table "products" {
        column "id" type="serial" primary="true"
        column "code" type="varchar" len="50" unique="true"
        column "name" type="varchar" len="255"
    }

    entity "Product" {
        field "name" type="String"
    }
    "#;

    let schema = compile(src, None).expect("Should compile");

    // Check Database
    let db = schema.database.expect("Database should be present");
    assert_eq!(db.db_type, "postgres");
    assert_eq!(db.url, "env:DATABASE_URL");

    // Check Table
    assert!(schema.tables.contains_key("products"));
    let table = schema.tables.get("products").unwrap();
    assert_eq!(table.columns.len(), 3);

    let col_code = table.columns.iter().find(|c| c.name == "code").unwrap();
    assert_eq!(col_code.type_name, "varchar");
    assert!(col_code.unique);
    assert_eq!(col_code.props.get("len").map(|s| s.as_str()), Some("50"));
}
