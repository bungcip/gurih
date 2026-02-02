
use gurih_dsl::compiler::compile;
use gurih_plugins::hr::HrPlugin;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::{DataStore, MemoryDataStore};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_hr_plugin_custom_effects() {
    let kdl = r#"
    database type="sqlite" url=":memory:"

    workflow "Suspension" entity="Employee" field="status" {
        state "Active"
        state "Suspended"
        state "Eligible"

        transition "suspend" from="Active" to="Suspended" {
            effects {
                suspend_payroll "true"
            }
        }

        transition "activate_rank" from="Suspended" to="Eligible" {
            effects {
                update_rank_eligibility "true"
            }
        }
    }

    entity "Employee" {
        field:pk "id"
        field:string "status"
        field:bool "is_payroll_active" default="true"
        field:bool "rank_eligible" default="false"
    }
    "#;

    let schema = compile(kdl, None).unwrap();
    let datastore: Arc<dyn DataStore> = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(Arc::new(schema), datastore).with_plugins(vec![Box::new(HrPlugin)]);
    let ctx = RuntimeContext::system();

    // 1. Create Employee
    let emp_data = json!({
        "id": "1",
        "status": "Active",
        "is_payroll_active": true,
        "rank_eligible": false
    });

    let id = engine.create("Employee", emp_data, &ctx).await.unwrap();

    // 2. Suspend Payroll
    let suspend_update = json!({
        "status": "Suspended"
    });
    engine.update("Employee", &id, suspend_update, &ctx).await.unwrap();

    let emp = engine.read("Employee", &id).await.unwrap().unwrap();
    assert_eq!(emp.get("is_payroll_active").unwrap(), false);

    // 3. Update Rank Eligibility
    let rank_update = json!({
        "status": "Eligible"
    });
    engine.update("Employee", &id, rank_update, &ctx).await.unwrap();

    let emp = engine.read("Employee", &id).await.unwrap().unwrap();
    assert_eq!(emp.get("rank_eligible").unwrap(), true);
}
