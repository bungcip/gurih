use gurih_ir::{Schema, StateSchema, Symbol, Transition, WorkflowSchema};
use gurih_runtime::workflow::WorkflowEngine;
use serde_json::Value;

#[tokio::test]
async fn test_workflow_transitions() {
    let mut schema = Schema::default();
    let entity_name = Symbol::from("Order");
    let initial_state = Symbol::from("Draft");
    let state_submitted = Symbol::from("Submitted");
    let state_approved = Symbol::from("Approved");

    let workflow = WorkflowSchema {
        name: Symbol::from("OrderWorkflow"),
        entity: entity_name,
        field: Symbol::from("state"),
        initial_state,
        states: vec![
            StateSchema {
                name: initial_state,
                immutable: false,
            },
            StateSchema {
                name: state_submitted,
                immutable: false,
            },
            StateSchema {
                name: state_approved,
                immutable: false,
            },
        ],
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

    schema.workflows.insert(workflow.name.clone(), workflow);

    let engine = WorkflowEngine::new();

    // 1. Initial State
    assert_eq!(engine.get_initial_state(&schema, "Order"), Some("Draft".to_string()));

    // 2. Valid Transition
    assert!(
        engine
            .validate_transition(&schema, None, "Order", "Draft", "Submitted", &Value::Null)
            .await
            .is_ok()
    );

    // 3. Same State Transition (Always allowed)
    assert!(
        engine
            .validate_transition(&schema, None, "Order", "Draft", "Draft", &Value::Null)
            .await
            .is_ok()
    );

    // 4. Invalid Transition
    assert!(
        engine
            .validate_transition(&schema, None, "Order", "Draft", "Approved", &Value::Null)
            .await
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

#[tokio::test]
async fn test_missing_precondition_field() {
    use gurih_ir::{BinaryOperator, Expression, TransitionPrecondition};

    let mut schema = Schema::default();
    let entity_name = Symbol::from("Employee");
    let initial_state = Symbol::from("Junior");
    let state_senior = Symbol::from("Senior");

    let workflow = WorkflowSchema {
        name: Symbol::from("PromotionWorkflow"),
        entity: entity_name,
        field: Symbol::from("status"),
        initial_state,
        states: vec![
            StateSchema {
                name: initial_state,
                immutable: false,
            },
            StateSchema {
                name: state_senior,
                immutable: false,
            },
        ],
        transitions: vec![Transition {
            name: Symbol::from("Promote"),
            from: initial_state,
            to: state_senior,
            required_permission: None,
            preconditions: vec![TransitionPrecondition::Assertion(Expression::BinaryOp {
                left: Box::new(Expression::FunctionCall {
                    name: Symbol::from("years_of_service"),
                    args: vec![Expression::Field(Symbol::from("custom_join_date"))],
                }),
                op: BinaryOperator::Gte,
                right: Box::new(Expression::Literal(5.0)),
            })],
            effects: vec![],
        }],
    };

    schema.workflows.insert(workflow.name.clone(), workflow);

    let engine = WorkflowEngine::new();
    // Use an empty object instead of null to allow field lookup attempts
    let empty_data = serde_json::json!({});

    let result = engine
        .validate_transition(&schema, None, "Employee", "Junior", "Senior", &empty_data)
        .await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    // With dynamic evaluation, missing field returns Null, causing type mismatch in years_of_service
    assert!(err.to_string().contains("Evaluation Error"));
    // We can't check for field name in error msg because it's a value error now
}
