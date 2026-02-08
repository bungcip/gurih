use gurih_dsl::parser::parse;
use gurih_dsl::compiler::compile;
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

    let ast = parse(input, None).unwrap();
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

    match parse(&src, d.parent()) {
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
#[test]
fn test_parse_entity_user() {
    let input = r#"
        entity:user "User" {
            field:pk id
            field:string "username" unique=#true
            field:password "password"
            field:string "role"
        }
    "#;

    let ast = parse(input, None).expect("Should parse entity:user");
    assert_eq!(ast.entities.len(), 1);
    
    let user_entity = &ast.entities[0];
    assert_eq!(user_entity.name, "User");
    assert!(user_entity.options.is_user_entity, "Should be marked as user entity");
}

#[test]
fn test_compile_entity_user() {
    let input = r#"
        entity:user "User" {
            field:pk id
            field:string "username" unique=#true
            field:password "password"
            field:string "role"
        }
    "#;

    let schema = compile(input, None).expect("Should compile entity:user");
    
    // Find the User entity in the compiled schema
    let user_entity = schema.entities.get(&"User".into()).expect("Should have User entity");
    
    // Check that is_user_entity option is set
    assert_eq!(
        user_entity.options.get("is_user_entity"),
        Some(&"true".to_string()),
        "Should have is_user_entity option set to true"
    );
}

#[test]
fn test_compile_multiple_entity_user_fails() {
    let input = r#"
        entity:user "User" {
            field:pk id
            field:string "username" unique=#true
            field:password "password"
        }

        entity:user "AdminUser" {
            field:pk id
            field:string "username" unique=#true
            field:password "password"
        }
    "#;

    let result = compile(input, None);
    assert!(result.is_err(), "Should fail with multiple entity:user declarations");
    
    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(
            error_msg.contains("Only one entity:user is allowed"),
            "Error should mention only one entity:user allowed. Got: {}",
            error_msg
        );
    }
}

#[test]
fn test_entity_user_in_module() {
    let input = r#"
        module "Auth" {
            entity:user "User" {
                field:pk id
                field:string "username" unique=#true
                field:password "password"
                field:string "role"

                seed {
                    row username="admin" password="password" role="HRManager"
                    row username="user" password="password" role="Employee"
                }
            }
        }
    "#;

    let schema = compile(input, None).expect("Should compile entity:user in module");
    
    // Find the User entity in the compiled schema
    let user_entity = schema.entities.get(&"User".into()).expect("Should have User entity");
    
    // Check that is_user_entity option is set
    assert_eq!(
        user_entity.options.get("is_user_entity"),
        Some(&"true".to_string()),
        "Should have is_user_entity option set to true"
    );
}