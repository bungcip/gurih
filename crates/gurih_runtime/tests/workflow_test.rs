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

    schema.workflows.insert(workflow.name, workflow);

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

    schema.workflows.insert(workflow.name, workflow);

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

#[tokio::test]
async fn test_workflow_effects() {
    use gurih_ir::{EntitySchema, FieldSchema, FieldType, TransitionEffect};

    let mut schema = Schema::default();
    let entity_name = Symbol::from("Invoice");
    let initial_state = Symbol::from("Unpaid");
    let state_paid = Symbol::from("Paid");
    let is_paid_field = Symbol::from("is_paid");

    // Define Entity with boolean field
    let entity = EntitySchema {
        name: entity_name,
        table_name: Symbol::from("invoices"),
        fields: vec![FieldSchema {
            name: is_paid_field,
            field_type: FieldType::Boolean,
            required: false,
            unique: false,
            default: None,
            references: None,
            serial_generator: None,
            storage: None,
            resize: None,
            filetype: None,
        }],
        relationships: vec![],
        options: Default::default(),
        seeds: None,
    };
    schema.entities.insert(entity_name, entity);

    // Define Workflow with effects
    let workflow = WorkflowSchema {
        name: Symbol::from("InvoiceWorkflow"),
        entity: entity_name,
        field: Symbol::from("status"),
        initial_state,
        states: vec![
            StateSchema {
                name: initial_state,
                immutable: false,
            },
            StateSchema {
                name: state_paid,
                immutable: true,
            },
        ],
        transitions: vec![Transition {
            name: Symbol::from("Pay"),
            from: initial_state,
            to: state_paid,
            required_permission: None,
            preconditions: vec![],
            effects: vec![
                TransitionEffect::Notify(Symbol::from("user@example.com")),
                TransitionEffect::UpdateField {
                    field: is_paid_field,
                    value: "true".to_string(),
                },
            ],
        }],
    };
    schema.workflows.insert(workflow.name, workflow);

    let engine = WorkflowEngine::new();
    let entity_data = serde_json::json!({});

    let (updates, notifications, postings) = engine
        .apply_effects(&schema, "Invoice", "Unpaid", "Paid", &entity_data)
        .await;

    // Check Notifications
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0], "user@example.com");

    // Check Updates (specifically boolean conversion)
    let updates_obj = updates.as_object().unwrap();
    assert!(updates_obj.contains_key("is_paid"));
    assert_eq!(updates_obj["is_paid"], Value::Bool(true));

    // Postings should be empty
    assert!(postings.is_empty());
}

#[tokio::test]
async fn test_workflow_assertion_type_error() {
    use gurih_ir::{Expression, TransitionPrecondition};

    let mut schema = Schema::default();
    let entity_name = Symbol::from("Task");
    let initial_state = Symbol::from("Todo");
    let state_done = Symbol::from("Done");

    let workflow = WorkflowSchema {
        name: Symbol::from("TaskWorkflow"),
        entity: entity_name,
        field: Symbol::from("status"),
        initial_state,
        states: vec![
            StateSchema {
                name: initial_state,
                immutable: false,
            },
            StateSchema {
                name: state_done,
                immutable: false,
            },
        ],
        transitions: vec![Transition {
            name: Symbol::from("Finish"),
            from: initial_state,
            to: state_done,
            required_permission: None,
            preconditions: vec![TransitionPrecondition::Assertion(Expression::Literal(1.0))], // Not a boolean
            effects: vec![],
        }],
    };

    schema.workflows.insert(workflow.name, workflow);

    let engine = WorkflowEngine::new();
    let empty_data = serde_json::json!({});

    let result = engine
        .validate_transition(&schema, None, "Task", "Todo", "Done", &empty_data)
        .await;

    assert!(result.is_err());
    let err = result.err().unwrap();
    assert_eq!(
        err.to_string(),
        "Workflow Error: Assertion expression must evaluate to boolean"
    );
}
