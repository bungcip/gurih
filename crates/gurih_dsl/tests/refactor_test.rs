use gurih_dsl::compiler::compile;
use gurih_ir::{Expression, TransitionPrecondition};

#[test]
fn test_employee_status_desugaring() {
    let src = r#"
        employee_status "CPNS" for="Pegawai" field="status_pegawai" {
            can_transition_to "PNS" {
                requires {
                    min_years_of_service 1 from="tmt_cpns"
                    min_age 18 from="tanggal_lahir"
                    document "sk_pns"
                }
            }
        }
    "#;

    let schema = compile(src, None).expect("Compilation failed");

    // Check workflow was generated
    let workflow = schema
        .workflows
        .get(&"PegawaiStatusWorkflow".into())
        .expect("Workflow not found");
    assert_eq!(workflow.entity.as_str(), "Pegawai");
    assert_eq!(workflow.field.as_str(), "status_pegawai");

    // Check transition
    let transition = workflow
        .transitions
        .iter()
        .find(|t| t.from == "CPNS".into() && t.to == "PNS".into())
        .expect("Transition not found");

    // Check preconditions desugared to Assertions
    assert_eq!(transition.preconditions.len(), 3);

    let mut found_service = false;
    let mut found_age = false;
    let mut found_doc = false;

    for pre in &transition.preconditions {
        if let TransitionPrecondition::Assertion(expr) = pre {
            // We can't easily stringify generic Expression, but we can inspect structure
            // Or assume the string representation from compiler debug
            let dbg = format!("{:?}", expr);
            if dbg.contains("years_of_service") && dbg.contains("tmt_cpns") {
                found_service = true;
            }
            if dbg.contains("age") && dbg.contains("tanggal_lahir") {
                found_age = true;
            }
            if dbg.contains("is_set") && dbg.contains("sk_pns") {
                found_doc = true;
            }
        }
    }

    assert!(found_service, "Missing years_of_service assertion");
    assert!(found_age, "Missing age assertion");
    assert!(found_doc, "Missing document assertion");
}
