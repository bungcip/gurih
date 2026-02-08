use gurih_dsl::parser::parse;
use std::path::Path;

#[test]
fn test_siasn_workflow_parsing() {
    let path = Path::new("../../gurih-siasn/workflow.kdl");
    let content = std::fs::read_to_string(path).expect("Failed to read workflow.kdl");
    let ast = parse(&content, None).expect("Failed to parse workflow.kdl");

    assert_eq!(ast.workflows.len(), 3);

    // Check new workflow
    let tb_wf = ast
        .workflows
        .iter()
        .find(|w| w.name == "UsulanTugasBelajarWorkflow")
        .expect("Missing UsulanTugasBelajarWorkflow");
    assert_eq!(tb_wf.entity, "UsulanTugasBelajar");
}

#[test]
fn test_siasn_status_parsing() {
    let path = Path::new("../../gurih-siasn/status.kdl");
    let content = std::fs::read_to_string(path).expect("Failed to read status.kdl");
    let ast = parse(&content, None).expect("Failed to parse status.kdl");

    // We expect employee_statuses, not workflows (compiler handles conversion)
    assert!(!ast.employee_statuses.is_empty());

    let cpns_status = ast
        .employee_statuses
        .iter()
        .find(|s| s.status == "CPNS")
        .expect("Missing CPNS status");
    assert_eq!(cpns_status.entity, "Pegawai");
    assert_eq!(cpns_status.field, Some("status_pegawai".to_string()));

    let to_pns = cpns_status
        .transitions
        .iter()
        .find(|t| t.to == "PNS")
        .expect("Missing -> PNS");

    // Check effects
    let has_rank_eligibility = to_pns.effects.iter().any(|e| match e {
        gurih_dsl::ast::TransitionEffectDef::Custom { name, args, .. } => {
            name == "update_rank_eligibility" && args == &vec!["true".to_string()]
        }
        _ => false,
    });
    assert!(has_rank_eligibility, "Missing update_rank_eligibility effect");
}
