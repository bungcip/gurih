use gurih_dsl::compiler::compile;
use gurih_ir::TransitionPrecondition;

#[test]
fn test_employee_status_desugaring() {
    let src = r#"
        entity "Pegawai" {
            pk "id"
            field "status_pegawai" type="String"
            field "tmt_cpns" type="Date"
            field "tanggal_lahir" type="Date"
            field "sk_pns" type="String"
        }

        workflow "PegawaiStatusWorkflow" for="Pegawai" field="status_pegawai" {
            state "CPNS"
            state "PNS"
            transition "CPNS_to_PNS" from="CPNS" to="PNS" {
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
        match pre {
            TransitionPrecondition::Assertion(expr) => {
                let dbg = format!("{:?}", expr);
                if dbg.contains("is_set") && dbg.contains("sk_pns") {
                    found_doc = true;
                }
            }
            TransitionPrecondition::Custom { name, kwargs, .. } => {
                if name.as_str() == "min_years_of_service" {
                    if let Some(field) = kwargs.get("from") {
                        if field == "tmt_cpns" {
                            found_service = true;
                        }
                    }
                }
                if name.as_str() == "min_age" {
                    if let Some(field) = kwargs.get("from") {
                        if field == "tanggal_lahir" {
                            found_age = true;
                        }
                    }
                }
            }
        }
    }

    assert!(found_service, "Missing years_of_service assertion");
    assert!(found_age, "Missing age assertion");
    assert!(found_doc, "Missing document assertion");
}
