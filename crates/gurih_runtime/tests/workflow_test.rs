use gurih_ir::{Schema, Symbol, WorkflowSchema, StateSchema, Transition, TransitionPrecondition, TransitionEffect, Expression};
use gurih_runtime::workflow::WorkflowEngine;
use gurih_runtime::hr_plugin::HrPlugin;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_employee_transition_logic() {
    let mut workflows = HashMap::new();

    workflows.insert(Symbol::from("PegawaiStatusWorkflow"), WorkflowSchema {
        name: Symbol::from("PegawaiStatusWorkflow"),
        entity: Symbol::from("Pegawai"),
        field: Symbol::from("status"),
        initial_state: Symbol::from("CPNS"),
        states: vec![
            StateSchema { name: Symbol::from("CPNS"), immutable: false },
            StateSchema { name: Symbol::from("PNS"), immutable: false },
        ],
        transitions: vec![
            Transition {
                name: Symbol::from("cpns_to_pns"),
                from: Symbol::from("CPNS"),
                to: Symbol::from("PNS"),
                required_permission: None,
                preconditions: vec![
                    TransitionPrecondition::Assertion(
                         Expression::BinaryOp {
                            left: Box::new(Expression::FunctionCall {
                                name: Symbol::from("years_of_service"),
                                args: vec![Expression::Field(Symbol::from("join_date"))],
                            }),
                            op: gurih_ir::BinaryOperator::Gte,
                            right: Box::new(Expression::Literal(1.0)),
                        }
                    )
                ],
                effects: vec![],
            }
        ],
    });

    let schema = Schema {
        workflows,
        ..Default::default()
    };

    let engine = WorkflowEngine::new();

    // Case 1: Less than 1 year service
    // Use tomorrow's date to be sure
    let future_date = (chrono::Utc::now() + chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
    let employee_fresh = json!({
        "status": "CPNS",
        "join_date": future_date
    });

    let res = engine.validate_transition(
        &schema,
        None,
        "Pegawai",
        "CPNS",
        "PNS",
        &employee_fresh
    ).await;

    assert!(res.is_err(), "Should fail for fresh employee");

    // Case 2: More than 1 year service
    let past_date = "2020-01-01";
    let employee_experienced = json!({
        "status": "CPNS",
        "join_date": past_date
    });

    let res = engine.validate_transition(
        &schema,
        None,
        "Pegawai",
        "CPNS",
        "PNS",
        &employee_experienced
    ).await;

    assert!(res.is_ok(), "Should pass for experienced employee");
}

#[tokio::test]
async fn test_hr_plugin_effects() {
    let mut workflows = HashMap::new();

    workflows.insert(Symbol::from("EffectWorkflow"), WorkflowSchema {
        name: Symbol::from("EffectWorkflow"),
        entity: Symbol::from("Pegawai"),
        field: Symbol::from("status"),
        initial_state: Symbol::from("Active"),
        states: vec![
            StateSchema { name: Symbol::from("Active"), immutable: false },
            StateSchema { name: Symbol::from("Suspended"), immutable: false },
        ],
        transitions: vec![
            Transition {
                name: Symbol::from("suspend"),
                from: Symbol::from("Active"),
                to: Symbol::from("Suspended"),
                required_permission: None,
                preconditions: vec![],
                effects: vec![
                    TransitionEffect::Custom {
                        name: Symbol::from("suspend_payroll"),
                        args: vec![Expression::StringLiteral("true".to_string())],
                    }
                ],
            }
        ],
    });

    let schema = Schema {
        workflows,
        ..Default::default()
    };

    let engine = WorkflowEngine::new()
        .with_plugins(vec![Box::new(HrPlugin)]);

    let employee = json!({
        "status": "Active",
        "is_payroll_active": true
    });

    let (updates, _, _) = engine.apply_effects(
        &schema,
        "Pegawai",
        "Active",
        "Suspended",
        &employee
    ).await;

    let updates_obj = updates.as_object().expect("Updates should be object");
    assert_eq!(updates_obj.get("is_payroll_active"), Some(&json!(false)));
}
