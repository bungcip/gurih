use gurih_dsl::compiler::compile;
use gurih_plugins::hr::HrPlugin;
use gurih_runtime::context::RuntimeContext;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::{DataStore, MemoryDataStore};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_siasn_integration_workflow() {
    // 1. Locate and compile the Siasn App Schema
    let mut path = std::env::current_dir().unwrap();
    while !path.join("gurih-siasn").exists() {
        if let Some(parent) = path.parent() {
            path = parent.to_path_buf();
        } else {
            break;
        }
    }
    let base_path = path.join("gurih-siasn");

    let app_kdl_path = base_path.join("app.kdl");
    assert!(
        app_kdl_path.exists(),
        "Could not find gurih-siasn/app.kdl at {:?}",
        app_kdl_path
    );

    let src = std::fs::read_to_string(&app_kdl_path).expect("Failed to read app.kdl");
    let schema = compile(&src, Some(&base_path)).expect("Failed to compile Siasn schema");
    let schema_arc = Arc::new(schema);

    // 2. Setup Runtime
    let datastore: Arc<dyn DataStore> = Arc::new(MemoryDataStore::new());

    let engine = DataEngine::new(schema_arc.clone(), datastore).with_plugins(vec![Box::new(HrPlugin)]);
    let ctx = RuntimeContext::system(); // Admin context

    // 3. Create Pegawai (CPNS)
    let tmt_cpns = "2023-01-01";
    let emp_data = json!({
        "nip": "199001012023011001",
        "nama": "Budi Santoso",
        "status_pegawai": "CPNS",
        "tmt_cpns": tmt_cpns,
        "jenis_kelamin": "L", // Enum variant might need match exact string
        "agama": "Islam",
        "tempat_lahir": "Jakarta",
        "tanggal_lahir": "1990-01-01",
        "is_payroll_active": true
    });

    let id = engine
        .create("Pegawai", emp_data, &ctx)
        .await
        .expect("Failed to create Pegawai");

    // 4. Attempt transition CPNS -> PNS (Fail: Missing doc)
    let update_to_pns = json!({
        "status_pegawai": "PNS"
    });
    let res = engine.update("Pegawai", &id, update_to_pns.clone(), &ctx).await;
    assert!(res.is_err(), "Should fail without documents");

    // 5. Add Document & Validate TMT (Success)
    // Needs: min_years_of_service 1 from tmt_cpns, valid_effective_date tmt_pns, document sk_pns

    // Update fields first
    let doc_update = json!({
        "sk_pns": "doc_sk_pns.pdf",
        "tmt_pns": "2024-02-01" // > 1 year from 2023-01-01
    });
    engine
        .update("Pegawai", &id, doc_update, &ctx)
        .await
        .expect("Failed to update doc");

    // Transition
    engine
        .update("Pegawai", &id, update_to_pns, &ctx)
        .await
        .expect("Failed transition to PNS");

    // Verify State
    let emp = engine.read("Pegawai", &id).await.unwrap().unwrap();
    assert_eq!(emp.get("status_pegawai").unwrap(), "PNS");

    // 6. Transition PNS -> Nonaktif (Test new feature)
    // Requires: document "sk_pemberhentian"
    // Effects: update "is_payroll_active" "false"

    // Attempt fail
    let update_to_nonaktif = json!({
        "status_pegawai": "Nonaktif"
    });
    let res = engine.update("Pegawai", &id, update_to_nonaktif.clone(), &ctx).await;
    assert!(res.is_err(), "Should fail without sk_pemberhentian");

    // Add doc
    engine
        .update("Pegawai", &id, json!({"sk_pemberhentian": "sk_stop.pdf"}), &ctx)
        .await
        .unwrap();

    // Transition
    engine
        .update("Pegawai", &id, update_to_nonaktif, &ctx)
        .await
        .expect("Failed transition to Nonaktif");

    // Verify Effect
    let emp = engine.read("Pegawai", &id).await.unwrap().unwrap();
    assert_eq!(emp.get("status_pegawai").unwrap(), "Nonaktif");

    // Check effect
    // The effect sets is_payroll_active to false.
    // Check if the field is present and false.
    let payroll_active = emp.get("is_payroll_active");
    assert_eq!(
        payroll_active,
        Some(&json!(false)),
        "Payroll should be suspended (false)"
    );
}
