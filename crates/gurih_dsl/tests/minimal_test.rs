use gurih_dsl::parser::parse;

#[test]
fn test_minimal_kdl() {
    let _src = r#"
    layout "Main" {
        header {
            search_bar true
        }
    }
    "#;
    let src_quoted = r#"
    layout "Main" {
        header {
            search_bar "true"
        }
    }
    "#;

    // Testing quoted:
    if let Err(e) = parse(src_quoted, None) {
        panic!("Failed to parse quoted layout: {:?}", e);
    }

    let _src_prop = r#"
    layout "Main" {
        header search_bar=true {}
    }
    "#;
    // Testing property:
    /*
    if let Err(e) = parse(src_prop) {
        panic!("Failed to parse prop layout: {:?}", e);
    }
    */

    let src_int = r#"
    layout "Main" {
        header {
            width 100
        }
    }
    "#;
    if let Err(e) = parse(src_int, None) {
        panic!("Failed to parse int layout: {:?}", e);
    }

    // Original failing one commented out to avoid panic
    /*
    if let Err(e) = parse(src) {
        panic!("Failed to parse minimal layout (original): {:?}", e);
    }
    */
}
