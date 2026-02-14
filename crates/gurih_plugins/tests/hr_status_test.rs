use gurih_dsl::compile;
use gurih_plugins::hr::HrPlugin;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::init_datastore;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_employee_status_transition() {
    let src = r#"
    entity "Pegawai" {
        field:pk id
        field:string status
        field:boolean is_payroll_active default="true"
        field:boolean rank_eligible default="false"
        field:date tmt_cpns required=#false
        field:file sk_pns required=#false
    }

    employee_status "CPNS" for="Pegawai" {
        can_transition_to "PNS" {
            requires {
                min_years_of_service 1 from="tmt_cpns"
                document "sk_pns"
            }
            effects {
                update_rank_eligibility "true"
            }
        }
    }

    employee_status "PNS" for="Pegawai" {
        can_transition_to "Cuti" {
            effects {
                suspend_payroll "true"
            }
        }
    }
    "#;

    let schema = compile(src, None).expect("Failed to compile DSL");
    let schema_arc = Arc::new(schema);
    let datastore = init_datastore(schema_arc.clone(), None)
        .await
        .expect("Failed to init datastore");

    let data_engine = DataEngine::new(schema_arc.clone(), datastore.clone()).with_plugins(vec![Box::new(HrPlugin)]);

    // 1. Create CPNS Employee
    // Need to set tmt_cpns to 1 year ago to pass check
    let one_year_ago = chrono::Utc::now().date_naive() - chrono::Duration::days(366);
    let tmt_cpns = one_year_ago.format("%Y-%m-%d").to_string();

    let payload = json!({
        "status": "CPNS",
        "tmt_cpns": tmt_cpns,
        "sk_pns": "some_file.pdf", // document exists
        "is_payroll_active": true,
        "rank_eligible": false
    });

    let id = data_engine
        .create("Pegawai", payload, &gurih_runtime::context::RuntimeContext::system())
        .await
        .expect("Failed to create Pegawai");

    // 2. Transition CPNS -> PNS (Should Succeed)
    // We update the status, WorkflowEngine should intercept and validate
    let update_payload = json!({
        "status": "PNS"
    });

    data_engine
        .update(
            "Pegawai",
            &id,
            update_payload,
            &gurih_runtime::context::RuntimeContext::system(),
        )
        .await
        .expect("Failed to transition to PNS");

    // Verify effects
    let employee = data_engine
        .read("Pegawai", &id, &gurih_runtime::context::RuntimeContext::system())
        .await
        .expect("Read failed")
        .expect("Employee not found");
    assert_eq!(employee.get("status"), Some(&json!("PNS")));
    assert_eq!(employee.get("rank_eligible"), Some(&json!(true))); // Effect applied

    // 3. Transition PNS -> Cuti (Should Succeed and suspend payroll)
    let update_cuti = json!({
        "status": "Cuti"
    });

    data_engine
        .update(
            "Pegawai",
            &id,
            update_cuti,
            &gurih_runtime::context::RuntimeContext::system(),
        )
        .await
        .expect("Failed to transition to Cuti");

    let employee_cuti = data_engine
        .read("Pegawai", &id, &gurih_runtime::context::RuntimeContext::system())
        .await
        .expect("Read failed")
        .expect("Employee not found");
    assert_eq!(employee_cuti.get("status"), Some(&json!("Cuti")));
    assert_eq!(employee_cuti.get("is_payroll_active"), Some(&json!(false))); // Effect applied
}

#[tokio::test]
async fn test_employee_status_transition_fail_precondition() {
    let src = r#"
    entity "Pegawai" {
        field:pk id
        field:string status
        field:date tmt_cpns required=#false
        field:file sk_pns required=#false
    }

    employee_status "CPNS" for="Pegawai" {
        can_transition_to "PNS" {
            requires {
                min_years_of_service 1 from="tmt_cpns"
            }
        }
    }
    "#;

    let schema = compile(src, None).expect("Failed to compile DSL");
    let schema_arc = Arc::new(schema);
    let datastore = init_datastore(schema_arc.clone(), None)
        .await
        .expect("Failed to init datastore");

    let data_engine = DataEngine::new(schema_arc.clone(), datastore.clone()).with_plugins(vec![Box::new(HrPlugin)]);

    // Create CPNS with recent TMT (less than 1 year)
    let today = chrono::Utc::now().date_naive().format("%Y-%m-%d").to_string();

    let payload = json!({
        "status": "CPNS",
        "tmt_cpns": today
    });

    let id = data_engine
        .create("Pegawai", payload, &gurih_runtime::context::RuntimeContext::system())
        .await
        .expect("Failed to create");

    // Attempt transition
    let update_payload = json!({
        "status": "PNS"
    });

    let result = data_engine
        .update(
            "Pegawai",
            &id,
            update_payload,
            &gurih_runtime::context::RuntimeContext::system(),
        )
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Transition condition not met")
            || err.to_string().contains("Invalid transition")
            || err.to_string().contains("Minimum years of service not met")
    );
}
