use gurih_dsl::compile;

#[test]
fn test_query_params_parsing() {
    let src = r#"
    query:flat "TestQuery" for="Account" {
        params "start_date" "end_date"
        select "name"
    }
    entity "Account" {
        field:pk id
        field:text "name"
    }
    "#;

    let schema = compile(src, None).expect("Failed to compile");
    let query = schema.queries.get(&"TestQuery".into()).expect("Query not found");

    assert_eq!(query.params.len(), 2);
    assert_eq!(query.params[0], "start_date".into());
    assert_eq!(query.params[1], "end_date".into());
}
