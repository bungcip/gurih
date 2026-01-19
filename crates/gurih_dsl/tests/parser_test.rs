use gurih_dsl::parser::parse;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_parse_golden_master() {
    // Locate the gurih-hr/gurih.kdl file
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("../../gurih-hr/gurih.kdl");
    
    let src = fs::read_to_string(&d).expect("Failed to read gurih.kdl");
    
    match parse(&src) {
        Ok(ast) => {
            println!("Successfully parsed AST!");
            println!("Modules: {}", ast.modules.len());
            println!("Entities: {}", ast.entities.len());
            println!("Workflows: {}", ast.workflows.len());
            println!("Routes: {}", ast.routes.len());
            println!("Layouts: {}", ast.layouts.len());
            println!("Icons: {}", ast.icons.len());
            
            // Basic assertions
            assert!(ast.layouts.len() > 0, "Should have layouts");
            if ast.entities.is_empty() {
                // Check inside modules
                let total_entities: usize = ast.modules.iter().map(|m| m.entities.len()).sum();
                println!("Total entities in modules: {}", total_entities);
                if total_entities == 0 {
                    panic!("Should have entities (checked modules too)");
                }
            } else {
                 if ast.entities.is_empty() {
                    panic!("Should have entities");
                }
            }assert!(ast.routes.len() > 0, "Should have routes");
        },
        Err(e) => {
            panic!("Failed to parse: {:?}", e);
        }
    }
}
