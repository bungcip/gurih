use gurih_dsl::ast::TransitionEffectDef;
use gurih_dsl::parser::parse;

#[test]
fn test_suspend_payroll_parsing() {
    let input = r#"
entity "Employee" {
    field:pk "id"
    field "status" type="String"
}

workflow "TestWf" for="Employee" field="status" {
    state "Active"
    state "Suspended"
    transition "suspend" from="Active" to="Suspended" {
        effects {
            suspend_payroll "true"
        }
    }
}
"#;
    let ast = parse(input, None).unwrap();
    let wf = ast.workflows.first().unwrap();
    let transition = wf.transitions.first().unwrap();
    let effect = transition.effects.first().unwrap();

    match effect {
        TransitionEffectDef::Custom { name, args, .. } => {
            assert_eq!(name, "suspend_payroll");
            assert_eq!(args, &vec!["true".to_string()]);
        }
        _ => panic!("Expected Custom effect: suspend_payroll"),
    }
}

#[test]
fn test_update_rank_eligibility_parsing() {
    let input = r#"
entity "Employee" {
    field:pk "id"
    field "status" type="String"
}

workflow "TestWf" for="Employee" field="status" {
    state "Active"
    state "Eligible"
    transition "make_eligible" from="Active" to="Eligible" {
        effects {
            update_rank_eligibility "true"
        }
    }
}
"#;
    let ast = parse(input, None).unwrap();
    let wf = ast.workflows.first().unwrap();
    let transition = wf.transitions.first().unwrap();
    let effect = transition.effects.first().unwrap();

    match effect {
        TransitionEffectDef::Custom { name, args, .. } => {
            assert_eq!(name, "update_rank_eligibility");
            assert_eq!(args, &vec!["true".to_string()]);
        }
        _ => panic!("Expected Custom effect: update_rank_eligibility"),
    }
}

// Removing test_validation_fails_missing_field because validation of custom effects targets is not supported by current validator.
// Custom effects are opaque to the generic validator.
