use gurih_ir::{EntitySchema, FieldSchema, FieldType, Schema};
use gurih_runtime::form::FormEngine;
use std::collections::HashMap;

#[test]
fn test_default_form_label_generation() {
    let mut schema = Schema::default();

    let fields = vec![
        FieldSchema {
            name: "employee_id".to_string(),
            field_type: FieldType::String,
            required: false,
            unique: false,
            default: None,
            references: None,
            serial: None,
        },
        FieldSchema {
            name: "enrolled_at".to_string(),
            field_type: FieldType::Date,
            required: false,
            unique: false,
            default: None,
            references: None,
            serial: None,
        },
        FieldSchema {
            name: "sync_date".to_string(),
            field_type: FieldType::DateTime,
            required: false,
            unique: false,
            default: None,
            references: None,
            serial: None,
        },
    ];

    let entity = EntitySchema {
        name: "TestEntity".to_string(),
        fields,
        relationships: vec![],
        options: HashMap::new(),
    };

    schema.entities.insert("TestEntity".to_string(), entity);

    let engine = FormEngine::new();
    let form_json = engine.generate_default_form(&schema, "TestEntity").expect("Failed to generate form");

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
