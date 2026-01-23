use gurih_dsl::compile;
use gurih_runtime::datastore::{DataStore, MemoryDataStore};
use gurih_runtime::faker::FakerEngine;
use std::sync::Arc;

#[tokio::test]
async fn test_faker_generation() {
    let kdl = r#"
    entity "Department" {
        field "id" type="Pk"
        field "name" type="Name"
    }

    entity "Employee" {
        field "id" type="Pk"
        field "full_name" type="Name"
        field "email" type="Email"

        belongs_to "Department"
    }
    "#;

    let schema = compile(kdl, None).expect("Schema compilation failed");
    let datastore: Arc<dyn DataStore> = Arc::new(MemoryDataStore::new());

    let faker = FakerEngine::new();
    faker
        .seed_entities(&schema, datastore.as_ref(), 10)
        .await
        .expect("Faker failed");

    // Verify Department count
    let count_dept = datastore
        .count("Department", std::collections::HashMap::new())
        .await
        .unwrap();
    assert_eq!(count_dept, 10);

    // Verify Employee count
    let count_emp = datastore
        .count("Employee", std::collections::HashMap::new())
        .await
        .unwrap();
    assert_eq!(count_emp, 10);

    // Verify Relationship (Employee has department_id)
    // FakerEngine handles implicit belongs_to by adding {rel_name}_id field.
    // We need to guess the relationship name. Usually "department" if target is "Department".

    let employees = datastore.list("Employee", None, None).await.unwrap();
    // Debug print keys to see what was inserted
    if let Some(first) = employees.first() {
        println!("Employee keys: {:?}", first.as_object().unwrap().keys());
    }

    for emp in employees {
        // Try finding any field ending in _id that is not id
        let fk_key = emp.as_object().unwrap().keys().find(|k| k.ends_with("_id"));

        if let Some(key) = fk_key {
            let val = emp.get(key).unwrap();
            assert!(val.is_string(), "FK should be string (UUID)");
            let dept_id = val.as_str().unwrap();

            // Check if dept exists
            let dept = datastore.get("Department", dept_id).await.unwrap();
            assert!(dept.is_some(), "Referenced Department should exist for key {}", key);
        } else {
            panic!(
                "Employee should have department_id populated by FakerEngine for implicit relationship. Found keys: {:?}",
                emp.as_object().unwrap().keys()
            );
        }
    }
}
