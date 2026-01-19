use gurih_dsl::compiler::compile;
use miette::{GraphicalReportHandler, GraphicalTheme};

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

    let result = compile(src);
    assert!(
        result.is_err(),
        "Compilation should fail due to duplicate entity"
    );

    let err = result.unwrap_err();

    // Render the error without colors using miette's API directly
    // This avoids unsafe environment variable modification
    let handler = GraphicalReportHandler::new().with_theme(GraphicalTheme::unicode_nocolor());

    let mut s = String::new();
    handler
        .render_report(&mut s, &err)
        .expect("Failed to render report");

    insta::assert_snapshot!(s);
}
