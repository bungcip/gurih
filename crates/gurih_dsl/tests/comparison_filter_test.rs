use gurih_dsl::compiler::compile;
use gurih_ir::{BinaryOperator, Expression, Symbol};

#[test]
fn test_comparison_operators_in_filters() {
    let src = r#"
    entity Post {
        field:pk id
        title String
        views Integer
        published_at Date
    }

    query:flat FilteredPosts for="Post" {
        select "title"

        filter "views > 100"
        filter "views < 500"
        filter "views >= 10"
        filter "views <= 1000"
        filter "views == 50"
        filter "views != 0"
        filter "published_at < DATE('2000-01-01')"
    }
    "#;

    let schema = compile(src, None).expect("Compilation failed");
    let query_name = Symbol::from("FilteredPosts");
    let query = schema.queries.get(&query_name).expect("Query not found");
    let filters = &query.filters;

    assert_eq!(filters.len(), 7);

    // Check >
    if let Expression::BinaryOp { left, op, right } = &filters[0] {
        assert!(matches!(op, BinaryOperator::Gt));
        if let Expression::Field(f) = &**left {
            assert_eq!(f, &Symbol::from("views"));
        } else {
            panic!("Expected field on left");
        }
        if let Expression::Literal(n) = &**right {
            assert_eq!(*n, 100.0);
        } else {
            panic!("Expected literal on right");
        }
    } else {
        panic!("Expected BinaryOp for filter 0");
    }

    // Check <
    if let Expression::BinaryOp { op, .. } = &filters[1] {
        assert!(matches!(op, BinaryOperator::Lt));
    } else {
        panic!("Expected BinaryOp for filter 1");
    }

    // Check >=
    if let Expression::BinaryOp { op, .. } = &filters[2] {
        assert!(matches!(op, BinaryOperator::Gte));
    } else {
        panic!("Expected BinaryOp for filter 2");
    }

    // Check <=
    if let Expression::BinaryOp { op, .. } = &filters[3] {
        assert!(matches!(op, BinaryOperator::Lte));
    } else {
        panic!("Expected BinaryOp for filter 3");
    }

    // Check ==
    if let Expression::BinaryOp { op, .. } = &filters[4] {
        assert!(matches!(op, BinaryOperator::Eq));
    } else {
        panic!("Expected BinaryOp for filter 4");
    }

    // Check !=
    if let Expression::BinaryOp { op, .. } = &filters[5] {
        assert!(matches!(op, BinaryOperator::Neq));
    } else {
        panic!("Expected BinaryOp for filter 5");
    }

    // Check published_at < DATE('2000-01-01')
    if let Expression::BinaryOp { left, op, right } = &filters[6] {
        assert!(matches!(op, BinaryOperator::Lt));
        if let Expression::Field(f) = &**left {
            assert_eq!(f, &Symbol::from("published_at"));
        } else {
            panic!("Expected field on left");
        }
        if let Expression::FunctionCall { name, args } = &**right {
            assert_eq!(name, &Symbol::from("DATE"));
            assert_eq!(args.len(), 1);
            if let Expression::StringLiteral(s) = &args[0] {
                assert_eq!(s, "2000-01-01");
            } else {
                panic!("Expected string literal arg");
            }
        } else {
            panic!("Expected function call on right");
        }
    } else {
        panic!("Expected BinaryOp for filter 6");
    }
}
