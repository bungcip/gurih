use gurih_dsl::compile;

#[test]
fn test_compile_valid_schema() {
    let src = r#"
    entity "Book" {
        field "title" type="String" required="true"
        field "author" type="String"
    }

    workflow "BookPublishing" entity="Book" {
        state "Draft" initial="true"
        state "Published"
        transition "publish" from="Draft" to="Published"
    }
    "#;

    let schema = compile(src).expect("Should compile");
    assert!(schema.entities.contains_key("Book"));
    assert!(schema.workflows.contains_key("BookPublishing"));
}

#[test]
fn test_compile_invalid_schema() {
    let src = r#"
    entity "Book" {
        field "title" type="UnknownType"
    }
    "#;

    let result = compile(src);
    assert!(result.is_err());
}
