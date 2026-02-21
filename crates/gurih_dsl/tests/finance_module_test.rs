use gurih_dsl::compile;
use std::path::PathBuf;

#[test]
fn test_compile_finance_module() {
    let root = PathBuf::from("../../gurih-finance/gurih.kdl");
    // Ensure we run from workspace root or handle path correctly.
    // Tests run from crate dir usually.
    let path = if root.exists() {
        root
    } else {
        PathBuf::from("gurih-finance/gurih.kdl") // Fallback if running from root
    };

    // Check if file exists relative to where we are
    if !path.exists() {
        // Try absolute path resolution relative to CARGO_MANIFEST_DIR
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = PathBuf::from(manifest_dir).join("../../gurih-finance/gurih.kdl");
        if !path.exists() {
            panic!("Could not find gurih-finance/gurih.kdl at {:?}", path);
        }
    }

    let path = std::env::current_dir().unwrap().join("../../gurih-finance/gurih.kdl");
    let content = std::fs::read_to_string(&path).expect(&format!("Failed to read {:?}", path));
    let base = path.parent();

    match compile(&content, base) {
        Ok(schema) => {
            println!("Compile Success! Found {} entities.", schema.entities.len());

            // Validate critical features are present
            assert!(
                schema
                    .queries
                    .contains_key(&gurih_ir::Symbol::from("TrialBalanceQuery"))
            );
            assert!(
                schema
                    .queries
                    .contains_key(&gurih_ir::Symbol::from("BalanceSheetQuery"))
            );
            assert!(
                schema
                    .queries
                    .contains_key(&gurih_ir::Symbol::from("IncomeStatementQuery"))
            );
        }
        Err(e) => {
            panic!("Compile Error: {:?}", e);
        }
    }
}
