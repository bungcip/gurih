use gurih_dsl::compiler::compile;
use gurih_ir::{Symbol, TransitionPrecondition, TransitionEffect};

#[test]
fn test_employee_status_compilation() {
    let src = r#"
        employee_status "CPNS" for="Pegawai" {
            can_transition_to "PNS" {
                requires {
                    min_years_of_service 1
                    document "sk_pns"
                }
                effects {
                    update_rank_eligibility "true"
                    notify "unit_kepegawaian"
                }
            }
        }

        employee_status "PNS" for="Pegawai" {
            can_transition_to "Cuti" {
                requires {
                    document "surat_cuti"
                }
            }
        }
    "#;

    let schema = compile(src, None).expect("Should compile");

    // Check Workflow
    let workflow = schema.workflows.get(&Symbol::from("PegawaiStatusWorkflow")).expect("Workflow not found");
    assert_eq!(workflow.entity, Symbol::from("Pegawai"));
    assert_eq!(workflow.field, Symbol::from("status"));

    // Check States
    let states: Vec<String> = workflow.states.iter().map(|s| s.name.to_string()).collect();
    assert!(states.contains(&"CPNS".to_string()));
    assert!(states.contains(&"PNS".to_string()));
    assert!(states.contains(&"Cuti".to_string()));

    // Check Transitions
    let t1 = workflow.transitions.iter().find(|t| t.from == Symbol::from("CPNS") && t.to == Symbol::from("PNS")).expect("CPNS->PNS missing");

    // Check Preconditions
    assert_eq!(t1.preconditions.len(), 2);

    // min_years_of_service 1 -> Assertion
    // document "sk_pns" -> Assertion(is_set("sk_pns"))

    // Note: Parsing order might vary depending on ast node order, but here it's sequential.

    let has_years = t1.preconditions.iter().any(|p| {
        match p {
            TransitionPrecondition::Assertion(expr) => {
                let s = format!("{:?}", expr);
                s.contains("years_of_service")
            },
            _ => false,
        }
    });
    assert!(has_years, "Expected years_of_service assertion");

    let has_doc = t1.preconditions.iter().any(|p| {
        match p {
            TransitionPrecondition::Assertion(expr) => {
                let s = format!("{:?}", expr);
                s.contains("is_set")
            },
            _ => false,
        }
    });
    assert!(has_doc, "Expected document (is_set) assertion");

    // Check Effects
    let has_rank = t1.effects.iter().any(|e| {
        match e {
            TransitionEffect::Custom { name, .. } => name == &Symbol::from("update_rank_eligibility"),
            _ => false,
        }
    });
    assert!(has_rank, "update_rank_eligibility missing");

    let has_notify = t1.effects.iter().any(|e| {
        match e {
            TransitionEffect::Notify(t) => t == &Symbol::from("unit_kepegawaian"),
            _ => false,
        }
    });
    assert!(has_notify, "notify missing");
}
