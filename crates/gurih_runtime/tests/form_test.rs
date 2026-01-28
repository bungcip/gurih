use gurih_ir::{EntitySchema, FieldSchema, FieldType, Schema, Symbol};
use gurih_runtime::form::FormEngine;
use std::collections::HashMap;

#[test]
fn test_default_form_label_generation() {
    let mut schema = Schema::default();

    let fields = vec![
        FieldSchema {
            name: Symbol::from("employee_id"),
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
            name: Symbol::from("enrolled_at"),
            field_type: FieldType::Date,
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
            name: Symbol::from("sync_date"),
            field_type: FieldType::Timestamp,
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

    let entity_id = Symbol::from("TestEntity");
    let entity = EntitySchema {
        name: entity_id,
        table_name: Symbol::from("test_entity"),
        fields,
        relationships: vec![],
        options: HashMap::new(),
        seeds: None,
    };

    schema.entities.insert(entity_id, entity);

    let engine = FormEngine::new();
    let form_json = engine
        .generate_default_form(&schema, entity_id)
        .expect("Failed to generate form");

    // Helper to find field by name
    let find_field = |name: &str| {
        let layout = form_json["layout"].as_array().unwrap();
        let section = &layout[0]; // "General" section
        let fields = section["fields"].as_array().unwrap();
        fields.iter().find(|f| f["name"].as_str() == Some(name)).cloned()
    };

    let f_employee = find_field("employee_id").expect("employee_id not found");
    let f_enrolled = find_field("enrolled_at").expect("enrolled_at not found");
    let f_sync = find_field("sync_date").expect("sync_date not found");

    assert_eq!(f_employee["label"], "Employee ID");
    assert_eq!(f_enrolled["label"], "Enrolled At");
    assert_eq!(f_sync["label"], "Sync Date");
}
