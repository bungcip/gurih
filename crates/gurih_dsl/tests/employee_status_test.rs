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
    // Check if states exist (names might differ slightly due to casing or defaults, but let's assume Symbol matches)
    // The compiler collects all states.
    // "pns", "cuti", "aktif" should be states.
    let states: Vec<String> = workflow.states.iter().map(|s| s.name.to_string()).collect();
    assert!(states.contains(&"pns".to_string()));
    assert!(states.contains(&"cuti".to_string()));
    assert!(states.contains(&"aktif".to_string()));

    // Check pns -> cuti transition
    let pns_to_cuti = workflow
        .transitions
        .iter()
        .find(|t| t.from == Symbol::from("pns") && t.to == Symbol::from("cuti"));
    assert!(pns_to_cuti.is_some());
    let t1 = pns_to_cuti.unwrap();

    // Check preconditions
    // Document "surat_cuti" -> Assertion(is_set(surat_cuti))
    assert!(
        t1.preconditions.iter().any(|p| {
            if let TransitionPrecondition::Assertion(gurih_ir::Expression::FunctionCall { name, args }) = p {
                if name.as_str() == "is_set" {
                    if let gurih_ir::Expression::Field(f) = &args[0] {
                        return f.as_str() == "surat_cuti";
                    }
                }
            }
            false
        }),
        "Missing document precondition"
    );

    // MinYearsOfService 1 -> Assertion(years_of_service(join_date) >= 1)
    assert!(
        t1.preconditions.iter().any(|p| {
             if let TransitionPrecondition::Assertion(gurih_ir::Expression::BinaryOp { left, op, right }) = p {
                 if let gurih_ir::Expression::FunctionCall { name, .. } = &**left {
                     if name.as_str() == "years_of_service" {
                         if let gurih_ir::BinaryOperator::Gte = op {
                             if let gurih_ir::Expression::Literal(n) = &**right {
                                 return *n == 1.0;
                             }
                         }
                     }
                 }
             }
             false
        }),
        "Missing min_years_of_service precondition"
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
