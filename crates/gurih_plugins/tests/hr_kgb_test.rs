use chrono::{Datelike, Local};
use gurih_dsl::compile;
use gurih_plugins::hr::HrPlugin;
use gurih_runtime::data::DataEngine;
use gurih_runtime::datastore::init_datastore;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_kgb_eligibility_success() {
    let src = r#"
    entity "Pegawai" {
        field:pk id
        field:string nama
        field:date tmt_golongan required=#false
        field:date tmt_kgb required=#false
        has_many "riwayat_skp" "RiwayatSKP"
    }

    entity "RiwayatSKP" {
        field:pk id
        belongs_to "Pegawai"
        field:integer tahun
        field:string predikat
    }

    entity "PengajuanKGB" {
        field:pk id
        belongs_to "Pegawai"
        field:string status default="Draft"
    }

    workflow "KGBWorkflow" for="PengajuanKGB" field="status" {
        state "Draft" initial="true"
        state "Diajukan"

        transition "Submit" from="Draft" to="Diajukan" {
            requires {
                check_kgb_eligibility "pegawai"
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
    let ctx = gurih_runtime::context::RuntimeContext::system();

    // 1. Create Eligible Employee (TMT > 2 years ago)
    let two_years_ago = (Local::now().date_naive() - chrono::Duration::days(366 * 2 + 10))
        .format("%Y-%m-%d")
        .to_string();

    let peg_payload = json!({
        "nama": "Budi",
        "tmt_golongan": two_years_ago,
        "tmt_kgb": two_years_ago
    });
    let peg_id = data_engine
        .create("Pegawai", peg_payload, &ctx)
        .await
        .expect("Create Pegawai failed");

    // 2. Create SKP records
    let current_year = Local::now().year();

    data_engine
        .create(
            "RiwayatSKP",
            json!({
                "pegawai": peg_id,
                "tahun": current_year - 1,
                "predikat": "Baik"
            }),
            &ctx,
        )
        .await
        .expect("Create SKP 1 failed");

    data_engine
        .create(
            "RiwayatSKP",
            json!({
                "pegawai": peg_id,
                "tahun": current_year - 2,
                "predikat": "Sangat Baik"
            }),
            &ctx,
        )
        .await
        .expect("Create SKP 2 failed");

    // 3. Create Submission (Draft)
    let sub_payload = json!({
        "pegawai": peg_id,
        "status": "Draft"
    });
    let sub_id = data_engine
        .create("PengajuanKGB", sub_payload, &ctx)
        .await
        .expect("Create Submission failed");

    // 4. Attempt Transition
    let update_payload = json!({ "status": "Diajukan" });
    data_engine
        .update("PengajuanKGB", &sub_id, update_payload, &ctx)
        .await
        .expect("Transition failed but should succeed");
}

#[tokio::test]
async fn test_kgb_eligibility_fail_time() {
    let src = r#"
    entity "Pegawai" {
        field:pk id
        field:date tmt_golongan required=#false
        field:date tmt_kgb required=#false
    }
    entity "RiwayatSKP" {
        field:pk id
        belongs_to "Pegawai"
        field:integer tahun
        field:string predikat
    }
    entity "PengajuanKGB" {
        field:pk id
        belongs_to "Pegawai"
        field:string status default="Draft"
    }
    workflow "KGBWorkflow" for="PengajuanKGB" field="status" {
        state "Draft" initial="true"
        state "Diajukan"
        transition "Submit" from="Draft" to="Diajukan" { requires { check_kgb_eligibility "pegawai" } }
    }
    "#;
    let schema = compile(src, None).expect("Compile failed");
    let schema_arc = Arc::new(schema);
    let datastore = init_datastore(schema_arc.clone(), None).await.expect("Init DB failed");
    let data_engine = DataEngine::new(schema_arc.clone(), datastore.clone()).with_plugins(vec![Box::new(HrPlugin)]);
    let ctx = gurih_runtime::context::RuntimeContext::system();

    // Recent TMT
    let recent = Local::now().date_naive().format("%Y-%m-%d").to_string();
    let peg_id = data_engine
        .create("Pegawai", json!({"tmt_golongan": recent}), &ctx)
        .await
        .expect("Create Peg failed");

    // Add valid SKP
    let current_year = Local::now().year();
    data_engine
        .create(
            "RiwayatSKP",
            json!({"pegawai": peg_id, "tahun": current_year - 1, "predikat": "Baik"}),
            &ctx,
        )
        .await
        .unwrap();
    data_engine
        .create(
            "RiwayatSKP",
            json!({"pegawai": peg_id, "tahun": current_year - 2, "predikat": "Baik"}),
            &ctx,
        )
        .await
        .unwrap();

    let sub_id = data_engine
        .create("PengajuanKGB", json!({"pegawai": peg_id, "status": "Draft"}), &ctx)
        .await
        .unwrap();

    let res = data_engine
        .update("PengajuanKGB", &sub_id, json!({"status": "Diajukan"}), &ctx)
        .await;
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("Belum memenuhi syarat 2 tahun"));
}

#[tokio::test]
async fn test_kgb_eligibility_fail_skp() {
    let src = r#"
    entity "Pegawai" {
        field:pk id
        field:date tmt_golongan required=#false
        field:date tmt_kgb required=#false
    }
    entity "RiwayatSKP" {
        field:pk id
        belongs_to "Pegawai"
        field:integer tahun
        field:string predikat
    }
    entity "PengajuanKGB" {
        field:pk id
        belongs_to "Pegawai"
        field:string status default="Draft"
    }
    workflow "KGBWorkflow" for="PengajuanKGB" field="status" {
        state "Draft" initial="true"
        state "Diajukan"
        transition "Submit" from="Draft" to="Diajukan" { requires { check_kgb_eligibility "pegawai" } }
    }
    "#;
    let schema = compile(src, None).expect("Compile failed");
    let schema_arc = Arc::new(schema);
    let datastore = init_datastore(schema_arc.clone(), None).await.expect("Init DB failed");
    let data_engine = DataEngine::new(schema_arc.clone(), datastore.clone()).with_plugins(vec![Box::new(HrPlugin)]);
    let ctx = gurih_runtime::context::RuntimeContext::system();

    // Old TMT (Valid)
    let old = (Local::now().date_naive() - chrono::Duration::days(366 * 3))
        .format("%Y-%m-%d")
        .to_string();
    let peg_id = data_engine
        .create("Pegawai", json!({"tmt_golongan": old}), &ctx)
        .await
        .expect("Create Peg failed");

    // Invalid SKP (Cukup)
    let current_year = Local::now().year();
    data_engine
        .create(
            "RiwayatSKP",
            json!({"pegawai": peg_id, "tahun": current_year - 1, "predikat": "Baik"}),
            &ctx,
        )
        .await
        .unwrap();
    data_engine
        .create(
            "RiwayatSKP",
            json!({"pegawai": peg_id, "tahun": current_year - 2, "predikat": "Cukup"}),
            &ctx,
        )
        .await
        .unwrap();

    let sub_id = data_engine
        .create("PengajuanKGB", json!({"pegawai": peg_id, "status": "Draft"}), &ctx)
        .await
        .unwrap();

    let res = data_engine
        .update("PengajuanKGB", &sub_id, json!({"status": "Diajukan"}), &ctx)
        .await;
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("Nilai SKP 2 tahun terakhir harus minimal 'Baik'"));
}
