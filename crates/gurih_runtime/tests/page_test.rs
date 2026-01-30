use gurih_ir::{EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_runtime::page::PageEngine;
use std::collections::HashMap;

#[test]
fn test_generate_page_config_fallback() {
    let mut schema = Schema::default();
    let entity_name = Symbol::from("MyEntity");

    let field1 = FieldSchema {
        name: Symbol::from("first_name"),
        field_type: FieldType::String,
        required: true,
        unique: false,
        default: None,
        references: None,
        serial_generator: None,
        storage: None,
        resize: None,
        filetype: None,
    };

    let field2 = FieldSchema {
        name: Symbol::from("age"),
        field_type: FieldType::Integer,
        required: false,
        unique: false,
        default: None,
        references: None,
        serial_generator: None,
        storage: None,
        resize: None,
        filetype: None,
    };

    let entity = EntitySchema {
        name: entity_name,
        table_name: Symbol::from("my_entity"),
        fields: vec![field1, field2],
        relationships: vec![],
        options: HashMap::new(),
        seeds: None,
    };

    schema.entities.insert(entity_name, entity);

    let engine = PageEngine::new();
    let config = engine.generate_page_config(&schema, entity_name).unwrap();

    assert_eq!(config["layout"], "TableView");
    assert_eq!(config["title"], "MyEntity");

    let columns = config["columns"].as_array().unwrap();
    assert_eq!(columns.len(), 2);

    // Check first column
    let col1 = &columns[0];
    assert_eq!(col1["key"], "first_name");
    assert_eq!(col1["label"], "First Name");
    assert_eq!(col1["type"], "String");

    // Check second column
    let col2 = &columns[1];
    assert_eq!(col2["key"], "age");
    assert_eq!(col2["label"], "Age");
    assert_eq!(col2["type"], "Integer");

    let actions = config["actions"].as_array().unwrap();
    assert_eq!(actions.len(), 3);
    assert_eq!(actions[0]["label"], "Create");
    assert_eq!(actions[1]["label"], "Edit");
    assert_eq!(actions[2]["label"], "Delete");
}
