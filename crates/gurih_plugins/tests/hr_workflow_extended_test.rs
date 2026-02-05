use gurih_dsl::compiler::compile;
use gurih_plugins::hr::HrPlugin;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::{DataStore, MemoryDataStore};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_effective_date_and_rank_eligibility() {
    let kdl = r#"
    database type="sqlite" url=":memory:"

    workflow "Promotion" entity="Employee" field="status" {
        state "Assistant"
        state "Associate"

        transition "promote" from="Assistant" to="Associate" {
            requires {
                valid_effective_date "tmt_promotion"
            }
            effects {
                update "rank_eligible" "true"
                update "last_promotion_date" "2024-01-01"
            }
        }
    }

    entity "Employee" {
        field:pk "id"
        field:string "status"
        field:date "tmt_promotion"
        field:bool "rank_eligible" default="false"
        field:date "last_promotion_date"
    }
    "#;

    let schema = compile(kdl, None).unwrap();
    let datastore: Arc<dyn DataStore> = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(Arc::new(schema), datastore).with_plugins(vec![Box::new(HrPlugin)]);
    let ctx = RuntimeContext::system();

    // 1. Create Employee
    let emp_data = json!({
        "id": "1",
        "status": "Assistant",
        "rank_eligible": false
    });

    let id = engine.create("Employee", emp_data, &ctx).await.unwrap();

    // 2. Attempt transition with invalid date format
    let invalid_date_update = json!({
        "status": "Associate",
        "tmt_promotion": "01-01-2024" // Wrong format (should be YYYY-MM-DD)
    });
    let res = engine.update("Employee", &id, invalid_date_update, &ctx).await;
    assert!(res.is_err());
    let err_msg = res.unwrap_err();
    assert!(err_msg.contains("Transition condition not met") || err_msg.contains("Invalid effective date for field"));

    // 3. Attempt transition with valid date
    let valid_update = json!({
        "status": "Associate",
        "tmt_promotion": "2024-01-01"
    });
    let res = engine.update("Employee", &id, valid_update, &ctx).await;
    assert!(res.is_ok());

    // 4. Verify effects
    let emp = engine.read("Employee", &id).await.unwrap().unwrap();

    // Status updated
    assert_eq!(emp.get("status").unwrap(), "Associate");
    // rank_eligible updated to true
    assert_eq!(emp.get("rank_eligible").unwrap(), true);
    // last_promotion_date updated by 'update' effect
    assert_eq!(emp.get("last_promotion_date").unwrap(), "2024-01-01");
}
