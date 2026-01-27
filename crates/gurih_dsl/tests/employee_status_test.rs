use gurih_dsl::compiler::compile;
use gurih_ir::{Symbol, TransitionEffect, TransitionPrecondition};

#[test]
fn test_employee_status_compilation() {
    let src = r#"
    employee_status "pns" {
      can_transition_to "cuti" {
        requires {
          document "surat_cuti"
          min_years_of_service 1
        }

        effects {
          suspend_payroll #true
          notify "unit_kepegawaian"
        }
      }
    }

    employee_status "cuti" {
      can_transition_to "aktif" {
        effects {
           suspend_payroll #false
        }
      }
    }
    "#;

    let schema = compile(src, None).expect("Failed to compile DSL");

    let workflow = schema
        .workflows
        .get(&Symbol::from("EmployeeStatusWorkflow"))
        .expect("Workflow not found");

    assert_eq!(workflow.entity, Symbol::from("Employee"));
    assert_eq!(workflow.field, Symbol::from("status"));

    // Check states
    use gurih_ir::StateSchema;
    assert!(workflow.states.contains(&StateSchema {
        name: Symbol::from("pns"),
        immutable: false
    }));
    assert!(workflow.states.contains(&StateSchema {
        name: Symbol::from("cuti"),
        immutable: false
    }));
    assert!(workflow.states.contains(&StateSchema {
        name: Symbol::from("aktif"),
        immutable: false
    }));

    // Check pns -> cuti transition
    let pns_to_cuti = workflow
        .transitions
        .iter()
        .find(|t| t.from == Symbol::from("pns") && t.to == Symbol::from("cuti"));
    assert!(pns_to_cuti.is_some());
    let t1 = pns_to_cuti.unwrap();

    // Check preconditions
    use gurih_ir::Expression;
    assert!(
        t1.preconditions
            .iter()
            .any(|p| matches!(p, TransitionPrecondition::Assertion(Expression::FunctionCall { name, args })
                if name.as_str() == "is_set" && matches!(&args[0], Expression::Field(f) if f.as_str() == "surat_cuti")))
    );
    assert!(
        t1.preconditions
            .iter()
            .any(|p| matches!(p, TransitionPrecondition::Assertion(Expression::BinaryOp { op, right, .. })
                if matches!(op, gurih_ir::BinaryOperator::Gte) && matches!(**right, Expression::Literal(l) if l == 1.0)))
    );

    // Check effects
    // suspend_payroll #true means active = false
    assert!(
        t1.effects
            .iter()
            .any(|e| matches!(e, TransitionEffect::UpdateField { field, value } if field == &Symbol::from("is_payroll_active") && value == "false"))
    );
    assert!(
        t1.effects
            .iter()
            .any(|e| matches!(e, TransitionEffect::Notify(target) if target == &Symbol::from("unit_kepegawaian")))
    );

    // Check cuti -> aktif transition
    let cuti_to_aktif = workflow
        .transitions
        .iter()
        .find(|t| t.from == Symbol::from("cuti") && t.to == Symbol::from("aktif"));
    assert!(cuti_to_aktif.is_some());
    let t2 = cuti_to_aktif.unwrap();

    // suspend_payroll #false means active = true
    assert!(
        t2.effects
            .iter()
            .any(|e| matches!(e, TransitionEffect::UpdateField { field, value } if field == &Symbol::from("is_payroll_active") && value == "true"))
    );
}
