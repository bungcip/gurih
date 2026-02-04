use gurih_dsl::compile;
use gurih_ir::Symbol;

#[test]
fn test_custom_status_field() {
    let src = r#"
    entity "Employee" {
        field:pk id
        field:string current_state
        field:date join_date
    }

    employee_status "Onboarding" for="Employee" field="current_state" {
        can_transition_to "Active" {
            requires {
                min_years_of_service 0
            }
        }
    }
    "#;

    let schema = compile(src, None).expect("Compilation failed");

    // Check if workflow is created
    let workflow = schema.workflows.get(&Symbol::from("EmployeeStatusWorkflow")).expect("Workflow not found");

    // Check if field is correct
    assert_eq!(workflow.field, Symbol::from("current_state"));
    assert_eq!(workflow.entity, Symbol::from("Employee"));

    // Check transitions
    let transition = workflow.transitions.iter().find(|t| t.from == Symbol::from("Onboarding") && t.to == Symbol::from("Active"));
    assert!(transition.is_some(), "Transition not found");
}

#[test]
fn test_default_status_field() {
    let src = r#"
    entity "Employee" {
        field:pk id
        field:string status
    }

    employee_status "Active" for="Employee" {
        can_transition_to "Inactive"
    }
    "#;

    let schema = compile(src, None).expect("Compilation failed");
    let workflow = schema.workflows.get(&Symbol::from("EmployeeStatusWorkflow")).expect("Workflow not found");
    assert_eq!(workflow.field, Symbol::from("status"));
}
