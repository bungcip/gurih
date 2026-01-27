use gurih_dsl::compile;
use gurih_ir::{Symbol, TransitionEffect, TransitionPrecondition};

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

    // Expect Assertion(is_set(field("approval_letter")))
    use gurih_ir::Expression;
    let is_correct = matches!(&trans.preconditions[0], TransitionPrecondition::Assertion(Expression::FunctionCall { name, args })
        if name.as_str() == "is_set" && matches!(&args[0], Expression::Field(f) if f.as_str() == "approval_letter")
    );
    assert!(is_correct, "Expected Assertion(is_set('approval_letter')), found {:?}", trans.preconditions[0]);
    assert!(
        matches!(trans.effects[0], TransitionEffect::UpdateField { ref field, ref value } if field.as_str() == "is_visible" && value == "true")
    );
}
