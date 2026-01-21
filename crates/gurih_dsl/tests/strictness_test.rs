use gurih_dsl::errors::CompileError;
use gurih_dsl::parser::parse;

#[test]
fn test_invalid_database_type() {
    let input = r#"
database {
    type "invalid_db"
    url "postgres://localhost/db"
}
"#;
    let result = parse(input, None);
    assert!(result.is_err());
    if let Err(CompileError::ParseError { message, .. }) = result {
        assert!(message.contains("Unsupported database type"));
    } else {
        panic!("Expected ParseError, got {:?}", result);
    }
}

#[test]
fn test_invalid_storage_driver() {
    let input = r#"
storage "my_storage" {
    driver "invalid_driver"
}
"#;
    let result = parse(input, None);
    assert!(result.is_err());
    if let Err(CompileError::ParseError { message, .. }) = result {
        assert!(message.contains("Unsupported storage driver"));
    } else {
        panic!("Expected ParseError, got {:?}", result);
    }
}

#[test]
fn test_invalid_widget_type() {
    let input = r#"
dashboard "my_db" {
    widget "my_widget" type="invalid_type" {
        label "My Widget"
    }
}
"#;
    let result = parse(input, None);
    assert!(result.is_err());
    if let Err(CompileError::ParseError { message, .. }) = result {
        assert!(message.contains("Unsupported widget type"));
    } else {
        panic!("Expected ParseError, got {:?}", result);
    }
}

#[test]
fn test_invalid_action_method() {
    let input = r#"
page "my_page" {
    datatable for="MyEntity" {
        action "MyAction" method="INVALID" {
            label "My Action"
        }
    }
}
"#;
    let result = parse(input, None);
    assert!(result.is_err());
    if let Err(CompileError::ParseError { message, .. }) = result {
        assert!(message.contains("Unsupported HTTP method"));
    } else {
        panic!("Expected ParseError, got {:?}", result);
    }
}

#[test]
fn test_invalid_action_step_type() {
    let input = r#"
action "my_action" {
    step "invalid:step" target="MyTarget"
}
"#;
    let result = parse(input, None);
    assert!(result.is_err());
    if let Err(CompileError::ParseError { message, .. }) = result {
        assert!(message.contains("Unknown action step type"));
    } else {
        panic!("Expected ParseError, got {:?}", result);
    }
}
