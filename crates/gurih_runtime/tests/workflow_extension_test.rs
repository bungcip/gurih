use gurih_ir::{Schema, StateSchema, Symbol, Transition, TransitionEffect, TransitionPrecondition, WorkflowSchema};
use gurih_runtime::workflow::WorkflowEngine;
use serde_json::json;

#[tokio::test]
async fn test_workflow_extensions() {
    let mut schema = Schema::default();
    let entity_name = Symbol::from("Pegawai");
    let state_cpns = Symbol::from("CPNS");
    let state_pns = Symbol::from("PNS");

    let workflow = WorkflowSchema {
        name: Symbol::from("PegawaiWorkflow"),
        entity: entity_name,
        field: Symbol::from("status_pegawai"),
        initial_state: state_cpns,
        states: vec![
            StateSchema {
                name: state_cpns,
                immutable: false,
            },
            StateSchema {
                name: state_pns,
                immutable: false,
            },
        ],
        transitions: vec![Transition {
            name: Symbol::from("AngkatPNS"),
            from: state_cpns,
            to: state_pns,
            required_permission: None,
            preconditions: vec![
                TransitionPrecondition::MinYearsOfService {
                    years: 1,
                    from_field: Some(Symbol::from("tmt_cpns")),
                },
                TransitionPrecondition::ValidEffectiveDate(Symbol::from("tmt_pns")),
            ],
            effects: vec![
                TransitionEffect::UpdateField {
                    field: Symbol::from("rank_eligible"),
                    value: "true".to_string(),
                },
                TransitionEffect::UpdateField {
                    field: Symbol::from("custom_field"),
                    value: "updated".to_string(),
                },
            ],
        }],
    };

    schema.workflows.insert(workflow.name, workflow);
    let engine = WorkflowEngine::new();

    // Test Case 1: Fail Min Years
    // Use today for TMT CPNS, so service is < 1 year
    let today = chrono::Utc::now().date_naive();
    let one_year_ago = today - chrono::Duration::days(366);
    let today_str = today.format("%Y-%m-%d").to_string();
    let one_year_ago_str = one_year_ago.format("%Y-%m-%d").to_string();

    let data_fail_years = json!({
        "tmt_cpns": today_str,
        "tmt_pns": today_str
    });

    let res_fail = engine
        .validate_transition(&schema, None, "Pegawai", "CPNS", "PNS", &data_fail_years)
        .await;
    assert!(res_fail.is_err());
    let err_msg = res_fail.unwrap_err();
    assert!(
        err_msg.to_string().contains("Minimum 1 years"),
        "Unexpected error: {}",
        err_msg
    );

    // Test Case 2: Fail Invalid Date
    let data_fail_date = json!({
        "tmt_cpns": one_year_ago_str,
        "tmt_pns": "invalid-date"
    });
    let res_fail_date = engine
        .validate_transition(&schema, None, "Pegawai", "CPNS", "PNS", &data_fail_date)
        .await;
    assert!(res_fail_date.is_err());
    let err_msg_date = res_fail_date.unwrap_err();
    assert!(
        err_msg_date.to_string().contains("valid date"),
        "Unexpected error: {}",
        err_msg_date
    );

    // Test Case 3: Success
    let data_success = json!({
        "tmt_cpns": one_year_ago_str,
        "tmt_pns": today_str
    });
    let res_success = engine
        .validate_transition(&schema, None, "Pegawai", "CPNS", "PNS", &data_success)
        .await;
    assert!(res_success.is_ok(), "Transition failed: {:?}", res_success.err());

    // Test Case 4: Effects
    let (updates, _notifications) = engine.apply_effects(&schema, "Pegawai", "CPNS", "PNS", &data_success);

    assert_eq!(updates.get("rank_eligible"), Some(&json!("true")));
    assert_eq!(updates.get("custom_field"), Some(&json!("updated")));
}
