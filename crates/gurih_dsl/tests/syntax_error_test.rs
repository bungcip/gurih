use gurih_dsl::compiler::compile;
use gurih_dsl::diagnostics::IntoDiagnostic;

#[test]
fn test_kdl_syntax_error() {
    let src = r#"
    node "foo" {
        unclosed_brace
    "#;

    let result = compile(src, None);
    assert!(result.is_err(), "Compilation should fail due to invalid KDL syntax");

    let err = result.unwrap_err();
    let diagnostics = err.into_diagnostic();

    assert!(!diagnostics.is_empty(), "Should produce at least one diagnostic");
    assert_eq!(diagnostics[0].code, Some("kdl_error".to_string()));
}
