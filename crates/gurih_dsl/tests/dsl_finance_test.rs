use gurih_dsl::compiler::compile;
use gurih_ir::{Symbol, TransitionPrecondition};

#[test]
fn test_finance_dsl_parsing() {
    let src = include_str!("test_finance_dsl.kdl");
    let schema = compile(src, None).expect("Should compile");

    // Verify Account Seeds
    let account = schema
        .entities
        .get(&Symbol::from("Account"))
        .expect("Account entity missing");
    let seeds = account.seeds.as_ref().expect("Seeds missing");
    assert_eq!(seeds.len(), 1);
    assert_eq!(seeds[0].get("name").map(|s| s.as_str()), Some("Cash"));
    assert_eq!(seeds[0].get("code").map(|s| s.as_str()), Some("101"));

    // Verify Workflow Preconditions
    let wf = schema
        .workflows
        .get(&Symbol::from("JournalWorkflow"))
        .expect("Workflow missing");
    let transition = wf
        .transitions
        .iter()
        .find(|t| t.name == Symbol::from("post"))
        .expect("Transition missing");

    let has_balanced = transition
        .preconditions
        .iter()
        .any(|p| matches!(p, TransitionPrecondition::Custom { name, .. } if name == &Symbol::from("balanced_transaction")));
    let has_period_open = transition
        .preconditions
        .iter()
        .any(|p| matches!(p, TransitionPrecondition::Custom { name, .. } if name == &Symbol::from("period_open")));

    assert!(has_balanced);
    assert!(has_period_open);
}
