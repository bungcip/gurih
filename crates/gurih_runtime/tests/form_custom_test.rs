use gurih_ir::{EntitySchema, FieldSchema, FieldType, FormSchema, FormSection, Schema, Symbol};
use gurih_runtime::form::FormEngine;
use std::collections::HashMap;

#[test]
fn test_custom_form_generation() {
    let mut schema = Schema::default();
    let entity_name = Symbol::from("WidgetTester");

    let fields = vec![
        FieldSchema {
            name: Symbol::from("description"),
            field_type: FieldType::Description,
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
            name: Symbol::from("avatar"),
            field_type: FieldType::Avatar,
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
            name: Symbol::from("price"),
            field_type: FieldType::Money,
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
            name: Symbol::from("secret"),
            field_type: FieldType::Password,
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
            name: Symbol::from("is_active"),
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
            name: Symbol::from("category"),
            field_type: FieldType::Enum(vec![Symbol::from("A"), Symbol::from("B")]),
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
            name: Symbol::from("script"),
            field_type: FieldType::Code,
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
            name: Symbol::from("custom"),
            field_type: FieldType::Custom("MyType".to_string()),
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
        name: entity_name,
        table_name: Symbol::from("widget_tester"),
        fields,
        relationships: vec![],
        options: HashMap::new(),
        seeds: None,
    };

    schema.entities.insert(entity_name, entity);

    let form_name = Symbol::from("WidgetForm");
    let form = FormSchema {
        name: form_name,
        entity: entity_name,
        sections: vec![FormSection {
            title: "Main".to_string(),
            fields: vec![
                Symbol::from("description"),
                Symbol::from("avatar"),
                Symbol::from("price"),
                Symbol::from("secret"),
                Symbol::from("is_active"),
                Symbol::from("category"),
                Symbol::from("script"),
                Symbol::from("custom"),
            ],
        }],
    };

    schema.forms.insert(form_name, form);

    let engine = FormEngine::new();
    let result = engine.generate_ui_schema(&schema, form_name);

    assert!(result.is_ok());
    let ui_schema = result.unwrap();

    assert_eq!(ui_schema["name"], "WidgetForm");
    assert_eq!(ui_schema["entity"], "WidgetTester");

    let layout = ui_schema["layout"].as_array().unwrap();
    assert_eq!(layout.len(), 1);
    let fields = layout[0]["fields"].as_array().unwrap();

    let find_widget = |name: &str| {
        fields
            .iter()
            .find(|f| f["name"].as_str() == Some(name))
            .map(|f| f["widget"].as_str().unwrap())
            .unwrap()
    };

    assert_eq!(find_widget("description"), "TextArea");
    assert_eq!(find_widget("avatar"), "ImageUpload");
    assert_eq!(find_widget("price"), "NumberInput");
    assert_eq!(find_widget("secret"), "PasswordInput");
    assert_eq!(find_widget("is_active"), "Checkbox");
    assert_eq!(find_widget("category"), "Select");
    assert_eq!(find_widget("script"), "CodeEditor");
    assert_eq!(find_widget("custom"), "TextInput");
}
