use gurih_dsl::compile;
use gurih_ir::{Ownership, RelationshipType, Symbol};
use std::path::Path;

#[test]
fn test_gurih_finance_immutability() {
    let finance_path = Path::new("../../gurih-finance");
    let main_file = finance_path.join("gurih.kdl");

    // Check if path exists (run test only if submodule is present)
    if !main_file.exists() {
        eprintln!("gurih-finance module not found at {:?}, skipping test.", main_file);
        return;
    }

    let src = std::fs::read_to_string(&main_file).expect("Failed to read gurih.kdl");
    let schema = compile(&src, Some(finance_path)).expect("Compilation of GurihFinance failed");

    // Verify JournalLine -> JournalEntry relationship ownership
    let journal_line = schema
        .entities
        .get(&Symbol::from("JournalLine"))
        .expect("JournalLine entity not found in compiled schema");

    let parent_rel = journal_line
        .relationships
        .iter()
        .find(|r| r.target_entity == Symbol::from("JournalEntry"))
        .expect("Relationship from JournalLine to JournalEntry not found");

    assert_eq!(
        parent_rel.rel_type,
        RelationshipType::BelongsTo,
        "JournalLine should belong to JournalEntry"
    );

    // This assertion confirms that the fix works for the actual module
    assert_eq!(
        parent_rel.ownership,
        Ownership::Composition,
        "JournalLine should have inferred Composition ownership due to JournalEntry's definition"
    );
}
