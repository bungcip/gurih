use gurih_dsl::parser::parse;
use std::path::Path;

#[test]
fn test_siasn_workflow_parsing() {
    let path = Path::new("../../gurih-siasn/workflow.kdl");
    let content = std::fs::read_to_string(path).expect("Failed to read workflow.kdl");
    let ast = parse(&content, None).expect("Failed to parse workflow.kdl");

    assert_eq!(ast.workflows.len(), 3);
    let wf = &ast.workflows[0];
    assert_eq!(wf.name, "PegawaiStatusWorkflow");
    assert_eq!(wf.entity, "Pegawai");
    assert_eq!(wf.field, "status_pegawai");

    // Check initial state
    let initial_state = wf.states.iter().find(|s| s.initial).expect("No initial state");
    assert_eq!(initial_state.name, "CPNS");

    // Check transitions
    let to_pns = wf
        .transitions
        .iter()
        .find(|t| t.from == "CPNS" && t.to == "PNS")
        .expect("Missing CPNS->PNS transition");

    // Check effects
    let has_rank_eligibility = to_pns.effects.iter().any(|e| match e {
        gurih_dsl::ast::TransitionEffectDef::Custom { name, args, .. } => {
            name == "update_rank_eligibility" && args == &vec!["true".to_string()]
        }
        _ => false,
    });
    assert!(has_rank_eligibility, "Missing update_rank_eligibility effect");
}
