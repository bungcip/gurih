use chrono::Utc;
use gurih_dsl::compiler::compile;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::{DataStore, MemoryDataStore};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_hr_workflow_rules() {
    let kdl = r#"
    database type="sqlite" url=":memory:"

    workflow "EmployeeStatus" entity="Employee" field="status" {
        state "pns"
        state "cuti"

        transition "pns_to_cuti" from="pns" to="cuti" {
            requires {
                document "surat_cuti"
                min_years_of_service 1
            }
            effects {
                suspend_payroll "true"
                notify "unit_kepegawaian"
            }
        }
    }

    entity "Employee" {
        field:pk "id"
        field:name "name"
        field:string "status"
        field:date "tmt_cpns"
        field:string "surat_cuti"
        field:bool "is_payroll_active" default="true"
    }
    "#;

    let schema = compile(kdl, None).unwrap();
    // MemoryDataStore::new() returns Self, not Arc. But DataEngine takes Arc<dyn DataStore>.
    // MemoryDataStore implements DataStore.
    // However, I need to cast it to DataStore.
    // Explicit type annotation helps.
    let datastore: Arc<dyn DataStore> = Arc::new(MemoryDataStore::new());
    let engine = DataEngine::new(Arc::new(schema), datastore);
    let ctx = RuntimeContext::system();

    // 1. Create Employee
    let emp_data = json!({
        "id": "1",
        "name": "Budi",
        "status": "pns",
        "tmt_cpns": "2024-01-01",
        "is_payroll_active": true
    });

    let id = engine.create("Employee", emp_data, &ctx).await.unwrap();

    // 2. Attempt invalid transition (missing doc)
    let update_data = json!({
        "status": "cuti"
    });
    let res = engine.update("Employee", &id, update_data.clone(), &ctx).await;
    assert!(res.is_err());
    let err_msg = res.unwrap_err();
    println!("Error 1: {}", err_msg);
    assert!(err_msg.contains("Missing required document: surat_cuti"));

    // 3. Add document
    let doc_update = json!({
        "surat_cuti": "doc123.pdf"
    });
    engine.update("Employee", &id, doc_update, &ctx).await.unwrap();

    // 4. Attempt transition (insufficient years)
    // Update join date to today (0 years)
    let now = Utc::now();
    let recent_join = now.format("%Y-%m-%d").to_string();
    engine
        .update("Employee", &id, json!({"tmt_cpns": recent_join}), &ctx)
        .await
        .unwrap();

    let res = engine.update("Employee", &id, update_data.clone(), &ctx).await;
    assert!(res.is_err());
    let err_msg = res.unwrap_err();
    println!("Error 2: {}", err_msg);
    assert!(err_msg.contains("years of service required"));

    // 5. Update join date to 2 years ago
    let old_join = (now.date_naive() - chrono::Duration::days(365 * 2))
        .format("%Y-%m-%d")
        .to_string();
    engine
        .update("Employee", &id, json!({"tmt_cpns": old_join}), &ctx)
        .await
        .unwrap();

    // 6. Attempt valid transition
    let res = engine.update("Employee", &id, update_data, &ctx).await;
    assert!(res.is_ok(), "Failed valid transition: {:?}", res.err());

    // 7. Verify side effects
    let emp_opt = engine.read("Employee", &id).await.unwrap();
    let emp = emp_opt.unwrap();
    assert_eq!(emp.get("status").unwrap(), "cuti");
    // "suspend_payroll true" -> active should be false
    assert_eq!(emp.get("is_payroll_active").unwrap(), false);

    // 8. Test Atomic Update (New Employee)
    let emp_atomic = json!({
        "id": "2",
        "name": "Siti",
        "status": "pns",
        "tmt_cpns": old_join, // Sufficient years
        "is_payroll_active": true
    });
    let id2 = engine.create("Employee", emp_atomic, &ctx).await.unwrap();

    // Try to update status AND upload doc in one go
    let atomic_update = json!({
        "status": "cuti",
        "surat_cuti": "doc_atomic.pdf"
    });
    let res = engine.update("Employee", &id2, atomic_update, &ctx).await;
    assert!(res.is_ok(), "Atomic update failed: {:?}", res.err());
}
