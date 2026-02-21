use gurih_dsl::compiler::compile;
use gurih_ir::{BinaryOperator, Expression};
use gurih_runtime::evaluator::evaluate;
use gurih_runtime::query_engine::{QueryEngine, QueryPlan};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_verify_user_example() {
    let src = r#"
    entity Book {
        field id type="Pk"
        field published_at type="Date"
    }

    query BookQuery for="Book" {
        query:flat
        filter "[published_at] < DATE('2000-01-01')"
        select "published_at"
    }
    "#;

    let schema = compile(src, None).expect("Failed to compile schema");
    let query = schema
        .queries
        .get(&gurih_ir::Symbol::from("BookQuery"))
        .expect("Query not found");

    // Verify filter expression structure
    assert_eq!(query.filters.len(), 1);
    let filter = &query.filters[0];

    match filter {
        Expression::BinaryOp { left, op, right } => {
            match op {
                BinaryOperator::Lt => (),
                _ => panic!("Expected Lt operator, found {:?}", op),
            }

            // Verify left is field
            if let Expression::Field(f) = &**left {
                assert_eq!(f.to_string(), "published_at");
            } else {
                panic!("Expected Field on left");
            }

            // Verify right is FunctionCall DATE
            if let Expression::FunctionCall { name, args } = &**right {
                assert_eq!(name.to_string(), "DATE");
                assert_eq!(args.len(), 1);
                if let Expression::StringLiteral(s) = &args[0] {
                    assert_eq!(s, "2000-01-01");
                } else {
                    panic!("Expected StringLiteral arg");
                }
            } else {
                panic!("Expected FunctionCall on right");
            }
        }
        _ => panic!("Expected BinaryOp"),
    }

    // Verify SQL Generation
    let runtime_params = HashMap::new();
    let strategy = QueryEngine::plan(&schema, "BookQuery", &runtime_params).expect("Failed to plan");

    if let QueryPlan::ExecuteSql { sql, .. } = &strategy.plans[0] {
        println!("Generated SQL: {}", sql);
        assert!(sql.contains("WHERE [published_at] < DATE("));
    } else {
        panic!("Expected ExecuteSql plan");
    }
}

#[test]
fn test_like_ilike_sql_generation() {
    let src = r#"
    entity Book {
        field id type="Pk"
        field title type="String"
    }
    query BookLikeQuery for="Book" {
        query:flat
        filter "[title] like 'Harry Potter%'"
        select "title"
    }
    query BookILikeQuery for="Book" {
        query:flat
        filter "[title] ilike '%stone%'"
        select "title"
    }
    "#;

    let schema = compile(src, None).expect("Failed to compile schema");

    // Check LIKE
    let query_like = schema
        .queries
        .get(&gurih_ir::Symbol::from("BookLikeQuery"))
        .expect("Query not found");
    assert_eq!(query_like.filters.len(), 1);
    if let Expression::BinaryOp { op, .. } = &query_like.filters[0] {
        match op {
            BinaryOperator::Like => (),
            _ => panic!("Expected Like operator, found {:?}", op),
        }
    } else {
        panic!("Expected BinaryOp");
    }

    let runtime_params = HashMap::new();
    let strategy = QueryEngine::plan(&schema, "BookLikeQuery", &runtime_params).expect("Failed to plan");
    if let QueryPlan::ExecuteSql { sql, .. } = &strategy.plans[0] {
        println!("Like SQL: {}", sql);
        assert!(sql.contains("LIKE"));
    }

    // Check ILIKE
    let query_ilike = schema
        .queries
        .get(&gurih_ir::Symbol::from("BookILikeQuery"))
        .expect("Query not found");
    if let Expression::BinaryOp { op, .. } = &query_ilike.filters[0] {
        match op {
            BinaryOperator::ILike => (),
            _ => panic!("Expected ILike operator, found {:?}", op),
        }
    }

    let strategy_ilike = QueryEngine::plan(&schema, "BookILikeQuery", &runtime_params).expect("Failed to plan");
    if let QueryPlan::ExecuteSql { sql, .. } = &strategy_ilike.plans[0] {
        println!("ILike SQL: {}", sql);
        // Default DB type is Sqlite -> maps ILIKE to LIKE
        assert!(sql.contains("LIKE") || sql.contains("ILIKE"));
    }
}

#[tokio::test]
async fn test_like_evaluation() {
    let expr = Expression::BinaryOp {
        left: Box::new(Expression::StringLiteral("Harry Potter".into())),
        op: BinaryOperator::Like,
        right: Box::new(Expression::StringLiteral("Harry%".into())),
    };
    let res = evaluate(&expr, &json!({}), None, None).await.unwrap();
    assert_eq!(res, json!(true));

    let expr_fail = Expression::BinaryOp {
        left: Box::new(Expression::StringLiteral("Lord of the Rings".into())),
        op: BinaryOperator::Like,
        right: Box::new(Expression::StringLiteral("Harry%".into())),
    };
    let res_fail = evaluate(&expr_fail, &json!({}), None, None).await.unwrap();
    assert_eq!(res_fail, json!(false));

    // Test wildcard inside
    let expr_mid = Expression::BinaryOp {
        left: Box::new(Expression::StringLiteral("Harry Potter".into())),
        op: BinaryOperator::Like,
        right: Box::new(Expression::StringLiteral("H%Potter".into())),
    };
    assert_eq!(evaluate(&expr_mid, &json!({}), None, None).await.unwrap(), json!(true));

    // Test _ wildcard
    let expr_one = Expression::BinaryOp {
        left: Box::new(Expression::StringLiteral("Cat".into())),
        op: BinaryOperator::Like,
        right: Box::new(Expression::StringLiteral("C_t".into())),
    };
    assert_eq!(evaluate(&expr_one, &json!({}), None, None).await.unwrap(), json!(true));
}

#[tokio::test]
async fn test_ilike_evaluation() {
    let expr = Expression::BinaryOp {
        left: Box::new(Expression::StringLiteral("HARRY POTTER".into())),
        op: BinaryOperator::ILike,
        right: Box::new(Expression::StringLiteral("harry%".into())),
    };
    let res = evaluate(&expr, &json!({}), None, None).await.unwrap();
    assert_eq!(res, json!(true));
}
