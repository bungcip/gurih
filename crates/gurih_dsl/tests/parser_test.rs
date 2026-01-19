use gurih_dsl::parser::parse;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_snake_case_to_title_case_label_generation() {
    let input = r#"
entity "TestEntity" {
    string "first_name"
    string "last_name"
}

page "test_page" {
    datatable for="TestEntity" {
        column "first_name"
        column "last_name"
        column "address"
    }
}
"#;

    let ast = parse(input).unwrap();
    let page = ast.pages.first().unwrap();

    if let gurih_dsl::ast::PageContent::Datatable(datatable) = &page.content {
        assert_eq!(datatable.columns[0].label, "First Name");
        assert_eq!(datatable.columns[1].label, "Last Name");
        assert_eq!(datatable.columns[2].label, "Address");
    } else {
        panic!("Expected datatable content");
    }
}

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
            assert!(!ast.layouts.is_empty(), "Should have layouts");
            if ast.entities.is_empty() {
                // Check inside modules
                let total_entities: usize = ast.modules.iter().map(|m| m.entities.len()).sum();
                println!("Total entities in modules: {}", total_entities);
                if total_entities == 0 {
                    panic!("Should have entities (checked modules too)");
                }
            } else if ast.entities.is_empty() {
                panic!("Should have entities");
            }
            assert!(!ast.routes.is_empty(), "Should have routes");
        }
        Err(e) => {
            panic!("Failed to parse: {:?}", e);
        }
    }
}
