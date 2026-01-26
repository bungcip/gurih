use gurih_dsl::compile;
use gurih_ir::{Symbol, TransitionEffect, TransitionPrecondition, Expression};

#[test]
fn test_employee_status_dsl() {
    let src = r#"
        employee_status "Draft" for="Document" field="state" {
            can_transition_to "Published" {
                requires {
                    document "approval_letter"
                }
                effects {
                    update "is_visible" "true"
                }
            }
        }

        employee_status "Published" for="Document" field="state" {
             can_transition_to "Archived"
        }
    "#;

    let schema = compile(src, None).expect("Failed to compile DSL");

    // Check if workflow exists
    let wf_name = Symbol::from("DocumentStatusWorkflow");
    assert!(
        schema.workflows.contains_key(&wf_name),
        "Workflow DocumentStatusWorkflow not found"
    );

    let wf = schema.workflows.get(&wf_name).unwrap();
    assert_eq!(wf.entity, Symbol::from("Document"));
    assert_eq!(wf.field, Symbol::from("state"));
    assert_eq!(wf.initial_state, Symbol::from("Draft")); // First defined

    // Check transitions
    let trans = wf
        .transitions
        .iter()
        .find(|t| t.from == Symbol::from("Draft") && t.to == Symbol::from("Published"))
        .expect("Transition missing");

    match &trans.preconditions[0] {
        TransitionPrecondition::Assertion(Expression::FunctionCall { name, args }) => {
             assert_eq!(name.as_str(), "is_set");
             match &args[0] {
                 Expression::Field(s) => assert_eq!(s.as_str(), "approval_letter"),
                 _ => panic!("Expected field arg"),
             }
        }
        _ => panic!("Expected Assertion(is_set)"),
    }

    assert!(
        matches!(trans.effects[0], TransitionEffect::UpdateField { ref field, ref value } if field.as_str() == "is_visible" && value == "true")
    );
}
