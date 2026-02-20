use gurih_ir::{EntitySchema, FieldSchema, FieldType, FormItem, FormSchema, FormSection, GridDef, RelationshipSchema, RelationshipType, Schema, Symbol, Ownership};
use gurih_runtime::form::FormEngine;
use std::collections::HashMap;

#[test]
fn test_form_grid_columns_respected() {
    let mut schema = Schema::default();

    // Parent Entity
    let parent_name = Symbol::from("Order");
    let parent_entity = EntitySchema {
        name: parent_name,
        table_name: Symbol::from("order"),
        fields: vec![
            FieldSchema { name: Symbol::from("id"), field_type: FieldType::Pk, required: true, unique: true, default: None, references: None, serial_generator: None, storage: None, resize: None, filetype: None },
        ],
        relationships: vec![
            RelationshipSchema { name: Symbol::from("items"), target_entity: Symbol::from("OrderItem"), rel_type: RelationshipType::HasMany, ownership: Ownership::Composition }
        ],
        options: HashMap::new(),
        seeds: None,
    };
    schema.entities.insert(parent_name, parent_entity);

    // Child Entity
    let child_name = Symbol::from("OrderItem");
    let child_entity = EntitySchema {
        name: child_name,
        table_name: Symbol::from("order_item"),
        fields: vec![
            FieldSchema { name: Symbol::from("id"), field_type: FieldType::Pk, required: true, unique: true, default: None, references: None, serial_generator: None, storage: None, resize: None, filetype: None },
            FieldSchema { name: Symbol::from("product"), field_type: FieldType::String, required: true, unique: false, default: None, references: None, serial_generator: None, storage: None, resize: None, filetype: None },
            FieldSchema { name: Symbol::from("quantity"), field_type: FieldType::Integer, required: true, unique: false, default: None, references: None, serial_generator: None, storage: None, resize: None, filetype: None },
            FieldSchema { name: Symbol::from("price"), field_type: FieldType::Money, required: true, unique: false, default: None, references: None, serial_generator: None, storage: None, resize: None, filetype: None },
        ],
        relationships: vec![
            RelationshipSchema { name: Symbol::from("order"), target_entity: parent_name, rel_type: RelationshipType::BelongsTo, ownership: Ownership::Reference }
        ],
        options: HashMap::new(),
        seeds: None,
    };
    schema.entities.insert(child_name, child_entity);

    // Form with Grid specifying columns
    let form_name = Symbol::from("OrderForm");
    let form = FormSchema {
        name: form_name,
        entity: parent_name,
        sections: vec![FormSection {
            title: "Items".to_string(),
            items: vec![
                FormItem::Grid(GridDef {
                    field: Symbol::from("items"),
                    columns: Some(vec![Symbol::from("product"), Symbol::from("quantity")]), // Exclude price
                }),
            ],
        }],
    };
    schema.forms.insert(form_name, form);

    let engine = FormEngine::new();
    let result = engine.generate_ui_schema(&schema, form_name).expect("Failed to generate UI schema");

    let layout = result["layout"].as_array().unwrap();
    let fields = layout[0]["fields"].as_array().unwrap();
    let grid = &fields[0];

    assert_eq!(grid["widget"], "InputGrid");

    let columns = grid["columns"].as_array().unwrap();

    // CURRENT BEHAVIOR: Ignores columns -> returns all fields (product, quantity, price) excluding id/parent ref
    // EXPECTED BEHAVIOR: Only product and quantity

    // Let's assert what we expect to fail currently
    let col_names: Vec<&str> = columns.iter().map(|c| c["name"].as_str().unwrap()).collect();

    // If it currently returns all, it will have "price".
    // If fixed, it won't.
    if col_names.contains(&"price") {
        panic!("Grid contains 'price' but it should be excluded! Columns found: {:?}", col_names);
    }
}
