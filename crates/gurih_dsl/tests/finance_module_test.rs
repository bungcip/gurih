use gurih_dsl::compiler::compile;
use std::path::PathBuf;
use std::fs;

#[test]
fn test_compile_finance_module() {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("../../gurih-finance/gurih.kdl");

    let src = fs::read_to_string(&d).expect("Failed to read gurih.kdl");
    let schema = compile(&src, d.parent()).expect("Should compile");

    // Check Account Seeds
    let account = schema.entities.get(&gurih_ir::Symbol::from("Account")).expect("Account entity should exist");
    let seeds = account.seeds.as_ref().expect("Account seeds should exist");
    assert!(!seeds.is_empty(), "Should have seeds from 'account' nodes");

    println!("Found {} account seeds", seeds.len());
}
