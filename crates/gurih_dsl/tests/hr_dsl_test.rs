use gurih_dsl::parser::parse;
use gurih_dsl::validator::Validator;
use gurih_dsl::ast::TransitionEffectDef;

#[test]
fn test_suspend_payroll_parsing() {
    let input = r#"
employee_status "Active" for="Employee" field="status" {
    can_transition_to "Suspended" {
        effects {
            suspend_payroll #true
        }
    }
}
"#;
    let ast = parse(input, None).unwrap();
    let wf = ast.workflows.first().unwrap();
    let transition = wf.transitions.first().unwrap();
    let effect = transition.effects.first().unwrap();

    match effect {
        TransitionEffectDef::UpdateField { field, value, .. } => {
            assert_eq!(field, "is_payroll_active");
            assert_eq!(value, "false"); // suspend=true -> active=false
        }
        _ => panic!("Expected UpdateField"),
    }
}

#[test]
fn test_update_rank_eligibility_parsing() {
    let input = r#"
employee_status "Active" for="Employee" field="status" {
    can_transition_to "Eligible" {
        effects {
            update_rank_eligibility #true
        }
    }
}
"#;
    let ast = parse(input, None).unwrap();
    let wf = ast.workflows.first().unwrap();
    let transition = wf.transitions.first().unwrap();
    let effect = transition.effects.first().unwrap();

    match effect {
        TransitionEffectDef::UpdateField { field, value, .. } => {
            assert_eq!(field, "rank_eligible");
            assert_eq!(value, "true");
        }
        _ => panic!("Expected UpdateField"),
    }
}

#[test]
fn test_validation_fails_missing_field() {
    let input = r#"
entity "Employee" {
    field:pk id
    // Missing is_payroll_active
    field:string "status"
}

employee_status "Active" for="Employee" field="status" {
    can_transition_to "Suspended" {
        effects {
            suspend_payroll #true
        }
    }
}
"#;
    let ast = parse(input, None).unwrap();
    let validator = Validator::new(input);
    let result = validator.validate(&ast);

    assert!(result.is_err());
    let err = result.err().unwrap();
    match err {
        gurih_dsl::errors::CompileError::ValidationError { message, .. } => {
            assert!(message.contains("Effect target field 'is_payroll_active' not found"));
        }
        _ => panic!("Expected ValidationError, got {:?}", err),
    }
}
