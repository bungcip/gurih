use gurih_ir::{Schema, Symbol, Transition, WorkflowSchema};
use gurih_runtime::workflow::WorkflowEngine;
use serde_json::Value;

#[test]
fn test_workflow_transitions() {
    let mut schema = Schema::default();
    let entity_name = Symbol::from("Order");
    let initial_state = Symbol::from("Draft");
    let state_submitted = Symbol::from("Submitted");
    let state_approved = Symbol::from("Approved");

    let workflow = WorkflowSchema {
        name: Symbol::from("OrderWorkflow"),
        entity: entity_name,
        field: Symbol::from("state"),
        initial_state: initial_state,
        states: vec![initial_state, state_submitted, state_approved],
        transitions: vec![
            Transition {
                name: Symbol::from("Submit"),
                from: initial_state,
                to: state_submitted,
                required_permission: None,
                preconditions: vec![],
                effects: vec![],
            },
            Transition {
                name: Symbol::from("Approve"),
                from: state_submitted,
                to: state_approved,
                required_permission: Some(Symbol::from("can_approve")),
                preconditions: vec![],
                effects: vec![],
            },
        ],
    };

    schema.workflows.insert(workflow.name, workflow);

    let engine = WorkflowEngine::new();

    // 1. Initial State
    assert_eq!(engine.get_initial_state(&schema, "Order"), Some("Draft".to_string()));

    // 2. Valid Transition
    assert!(
        engine
            .validate_transition(&schema, "Order", "Draft", "Submitted", &Value::Null)
            .is_ok()
    );

    // 3. Same State Transition (Always allowed)
    assert!(
        engine
            .validate_transition(&schema, "Order", "Draft", "Draft", &Value::Null)
            .is_ok()
    );

    // 4. Invalid Transition
    assert!(
        engine
            .validate_transition(&schema, "Order", "Draft", "Approved", &Value::Null)
            .is_err()
    );

    // 5. Transition with Permission
    let perm = engine.get_transition_permission(&schema, "Order", "Submitted", "Approved");
    assert_eq!(perm, Some("can_approve".to_string()));

    // 6. Transition without Permission
    let perm_none = engine.get_transition_permission(&schema, "Order", "Draft", "Submitted");
    assert_eq!(perm_none, None);

    // 7. Same state permission (None)
    let perm_same = engine.get_transition_permission(&schema, "Order", "Draft", "Draft");
    assert_eq!(perm_same, None);
}
