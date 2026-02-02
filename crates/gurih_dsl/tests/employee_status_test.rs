use gurih_dsl::compiler::compile;
use gurih_ir::{BinaryOperator, Expression, Symbol, TransitionEffect, TransitionPrecondition};

#[test]
fn test_employee_status_compilation() {
    let src = r#"
    entity "Employee" {
        field:pk "id"
        field "status" type="String"
        field "surat_cuti" type="String"
        field "join_date" type="Date"
        field "is_payroll_active" type="Boolean"
    }

    employee_status "pns" {
      can_transition_to "cuti" {
        requires {
          document "surat_cuti"
          min_years_of_service 1
        }

        effects {
          suspend_payroll "true"
          notify "unit_kepegawaian"
        }
      }
    }

    employee_status "cuti" {
      can_transition_to "aktif" {
        effects {
           suspend_payroll "false"
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
    // document "surat_cuti" -> Assertion(is_set(surat_cuti))
    assert!(t1.preconditions.iter().any(|p| {
        if let TransitionPrecondition::Assertion(Expression::FunctionCall { name, args }) = p {
            name.as_str() == "is_set"
                && args.len() == 1
                && matches!(&args[0], Expression::Field(s) if s.as_str() == "surat_cuti")
        } else {
            false
        }
    }));

    // min_years_of_service 1 -> Assertion(years_of_service(tmt_cpns) >= 1)
    assert!(t1.preconditions.iter().any(|p| {
        if let TransitionPrecondition::Assertion(Expression::BinaryOp { op, .. }) = p {
            matches!(op, BinaryOperator::Gte)
        } else {
            false
        }
    }));

    // Check effects
    // suspend_payroll true -> Custom("suspend_payroll", ["true"])
    assert!(
        t1.effects
            .iter()
            .any(|e| matches!(e, TransitionEffect::Custom { name, args }
                if name == &Symbol::from("suspend_payroll")
                && matches!(args.first(), Some(Expression::StringLiteral(s)) if s == "true")
            ))
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

    // suspend_payroll false -> Custom("suspend_payroll", ["false"])
    assert!(
        t2.effects
            .iter()
            .any(|e| matches!(e, TransitionEffect::Custom { name, args }
                if name == &Symbol::from("suspend_payroll")
                && matches!(args.first(), Some(Expression::StringLiteral(s)) if s == "false")
            ))
    );
}

#[test]
fn test_document_status_workflow() {
    let src = r#"
        entity "Document" {
        field:pk "id"
            field "state" type="String"
            field "approval_letter" type="String"
            field "is_visible" type="Boolean"
        }

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
    // assert_eq!(wf.initial_state, Symbol::from("Draft")); // Parser doesn't support initial yet for employee_status

    // Check transitions
    let trans = wf
        .transitions
        .iter()
        .find(|t| t.from == Symbol::from("Draft") && t.to == Symbol::from("Published"))
        .expect("Transition missing");

    assert!(matches!(
        &trans.preconditions[0],
        TransitionPrecondition::Assertion(Expression::FunctionCall { name, args })
        if name.as_str() == "is_set"
           && args.len() == 1
           && matches!(&args[0], Expression::Field(s) if s.as_str() == "approval_letter")
    ));
    assert!(
        matches!(trans.effects[0], TransitionEffect::UpdateField { ref field, ref value } if field.as_str() == "is_visible" && value == "true")
    );
}
