use gurih_ir::{Schema, StateSchema, Symbol, Transition, TransitionPrecondition, WorkflowSchema};
use gurih_runtime::plugins::FinancePlugin;
use gurih_runtime::workflow::WorkflowEngine;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_balanced_transaction() {
    let engine = WorkflowEngine::new().with_plugins(vec![Box::new(FinancePlugin)]);

    // Construct minimal schema with workflow
    let transitions = vec![Transition {
        name: Symbol::from("post"),
        from: Symbol::from("Draft"),
        to: Symbol::from("Posted"),
        required_permission: None,
        preconditions: vec![TransitionPrecondition::Custom {
            name: Symbol::from("balanced_transaction"),
            args: vec![],
        }],
        effects: vec![],
    }];

    let wf = WorkflowSchema {
        name: Symbol::from("JournalWF"),
        entity: Symbol::from("Journal"),
        field: Symbol::from("status"),
        initial_state: Symbol::from("Draft"),
        states: vec![
            StateSchema {
                name: Symbol::from("Draft"),
                immutable: false,
            },
            StateSchema {
                name: Symbol::from("Posted"),
                immutable: false,
            },
        ],
        transitions,
    };

    let mut workflows = HashMap::new();
    workflows.insert(Symbol::from("Journal"), wf);

    let schema = Schema {
        workflows,
        ..Schema::default()
    };

    // Test Balanced
    let data_balanced = json!({
        "status": "Draft",
        "lines": [
            { "debit": 100.0, "credit": 0.0 },
            { "debit": 0.0, "credit": 100.0 }
        ]
    });

    let res = engine
        .validate_transition(&schema, None, "Journal", "Draft", "Posted", &data_balanced)
        .await;
    assert!(res.is_ok(), "Balanced transaction should pass: {:?}", res.err());

    // Test Unbalanced
    let data_unbalanced = json!({
        "status": "Draft",
        "lines": [
            { "debit": 100.0, "credit": 0.0 },
            { "debit": 0.0, "credit": 50.0 }
        ]
    });

    let res = engine
        .validate_transition(&schema, None, "Journal", "Draft", "Posted", &data_unbalanced)
        .await;
    assert!(res.is_err(), "Unbalanced transaction should fail");
}
