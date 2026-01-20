use gurih_dsl::compiler::compile;
use gurih_dsl::diagnostics::{ErrorFormatter, IntoDiagnostic};

#[test]
fn test_duplicate_entity_error() {
    let src = r#"
    entity "Employee" {
        string "name"
    }

    entity "Employee" {
        string "another_field"
    }
    "#;

    let result = compile(src, None);
    assert!(result.is_err(), "Compilation should fail due to duplicate entity");

    let err = result.unwrap_err();
    let diagnostics = err.into_diagnostic();
    let formatter = ErrorFormatter { use_colors: false };

    let mut s = String::new();
    for diag in diagnostics {
        s.push_str(&formatter.format_diagnostic(&diag, src, "test.kdl"));
        s.push('\n');
    }

    insta::assert_snapshot!(s);
}

#[test]
fn test_unknown_top_level_node() {
    let src = r#"
    unknown_node "something"
    "#;

    let result = compile(src, None);
    assert!(result.is_err(), "Compilation should fail due to unknown top-level node");

    let err = result.unwrap_err();
    let diagnostics = err.into_diagnostic();
    let formatter = ErrorFormatter { use_colors: false };

    let mut s = String::new();
    for diag in diagnostics {
        s.push_str(&formatter.format_diagnostic(&diag, src, "test.kdl"));
        s.push('\n');
    }

    insta::assert_snapshot!(s);
}

#[test]
fn test_missing_argument_error() {
    let src = r#"
    entity
    "#;

    let result = compile(src, None);
    assert!(result.is_err(), "Compilation should fail due to missing argument");

    let err = result.unwrap_err();
    let diagnostics = err.into_diagnostic();
    let formatter = ErrorFormatter { use_colors: false };

    let mut s = String::new();
    for diag in diagnostics {
        s.push_str(&formatter.format_diagnostic(&diag, src, "test.kdl"));
        s.push('\n');
    }

    insta::assert_snapshot!(s);
}

#[test]
fn test_route_to_unrecognized_page() {
    let src = r#"
    page "HomePage" {
        content {
            none
        }
    }

    routes {
        route "/" to="HomePage"
        route "/unknown" to="UnknownPage"
    }
    "#;

    let result = compile(src, None);
    assert!(
        result.is_err(),
        "Compilation should fail due to route linking to unrecognized page"
    );

    let err = result.unwrap_err();
    let diagnostics = err.into_diagnostic();
    let formatter = ErrorFormatter { use_colors: false };

    let mut s = String::new();
    for diag in diagnostics {
        s.push_str(&formatter.format_diagnostic(&diag, src, "test.kdl"));
        s.push('\n');
    }

    insta::assert_snapshot!(s);
}

#[test]
fn test_duplicate_field_error() {
    let src = r#"
    entity "User" {
        string "email"
        string "email"
    }
    "#;

    let result = compile(src, None);
    assert!(result.is_err(), "Compilation should fail due to duplicate field name");

    let err = result.unwrap_err();
    let diagnostics = err.into_diagnostic();
    let formatter = ErrorFormatter { use_colors: false };

    let mut s = String::new();
    for diag in diagnostics {
        s.push_str(&formatter.format_diagnostic(&diag, src, "test.kdl"));
        s.push('\n');
    }

    insta::assert_snapshot!(s);
}
