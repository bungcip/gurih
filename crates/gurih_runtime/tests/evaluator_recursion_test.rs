use gurih_ir::{BinaryOperator, Expression};
use gurih_runtime::evaluator::evaluate;
use serde_json::json;

fn build_deep_expression(depth: usize) -> Expression {
    if depth == 0 {
        return Expression::Literal(1.0);
    }
    Expression::BinaryOp {
        left: Box::new(Expression::Literal(1.0)),
        op: BinaryOperator::Add,
        right: Box::new(build_deep_expression(depth - 1)),
    }
}

#[tokio::test]
async fn test_stack_overflow_protection() {
    // 300 > 250 (limit)
    // This should fail gracefully
    let depth = 300;

    let expr = build_deep_expression(depth);
    let ctx = json!({});

    let result = evaluate(&expr, &ctx, None, None).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(format!("{}", err).contains("recursion limit exceeded"));
}

#[tokio::test]
async fn test_valid_depth() {
    // 10 < 250 (limit)
    // This should succeed
    let depth = 10;

    let expr = build_deep_expression(depth);
    let ctx = json!({});

    let result = evaluate(&expr, &ctx, None, None).await;

    assert!(result.is_ok());
    // Sum of 11 ones = 11.0 (depth 0 is 1.0, + 10 additions)
    assert_eq!(result.unwrap(), json!(11.0));
}
