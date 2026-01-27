use gurih_dsl::compile;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::MemoryDataStore;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_rule_enforcement() {
    let kdl = r#"
    entity "Person" {
        field:pk id
        field:name "name"
        field:date "birth_date"
        options {
            create_permission "public"
        }
    }

    rule "MinAge" {
        on "Person:create"
        assert "age(birth_date) >= 18"
        message "Person must be at least 18 years old"
    }

    rule "MinAgeUpdate" {
        on "Person:update"
        assert "age(birth_date) >= 18"
        message "Person must be at least 18 years old updated"
    }
    "#;

    let schema = Arc::new(compile(kdl, None).expect("Failed to compile schema"));
    let datastore = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(schema, datastore);

    // Context with permission
    let ctx = RuntimeContext::system();

    // 1. Test Valid Create (Age >= 18)
    // Assuming current year is 2024, 2000-01-01 is 24 years old.
    let valid_data = json!({
        "name": "John Doe",
        "birth_date": "2000-01-01"
    });

    let res = engine.create("Person", valid_data.clone(), &ctx).await;
    assert!(res.is_ok(), "Valid person creation failed: {:?}", res.err());
    let id = res.unwrap();

    // 2. Test Invalid Create (Age < 18)
    // 2010-01-01 is ~14 years old.
    let invalid_data = json!({
        "name": "Kid",
        "birth_date": "2010-01-01"
    });

    let res = engine.create("Person", invalid_data, &ctx).await;
    assert!(res.is_err(), "Invalid person creation should fail");
    let err = res.err().unwrap();
    assert_eq!(err, "Person must be at least 18 years old");

    // 3. Test Invalid Update
    // Updating birth_date to be underage
    let update_invalid = json!({
        "birth_date": "2015-01-01"
    });

    let res = engine.update("Person", &id, update_invalid, &ctx).await;
    assert!(res.is_err(), "Invalid update should fail");
    let err = res.err().unwrap();
    assert_eq!(err, "Person must be at least 18 years old updated");

    // 4. Test Valid Update
    let update_valid = json!({
        "birth_date": "1990-01-01"
    });
    let res = engine.update("Person", &id, update_valid, &ctx).await;
    assert!(res.is_ok(), "Valid update failed: {:?}", res.err());
}
