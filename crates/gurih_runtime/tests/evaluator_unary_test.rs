use gurih_ir::{Expression, UnaryOperator};
use gurih_runtime::evaluator::evaluate;
use serde_json::json;

#[tokio::test]
async fn test_unary_operators() {
    // Test UnaryOperator::Not
    let expr_not_true = Expression::UnaryOp {
        op: UnaryOperator::Not,
        expr: Box::new(Expression::BoolLiteral(true)),
    };
    let ctx = json!({});
    let res = evaluate(&expr_not_true, &ctx, None, None).await.unwrap();
    assert_eq!(res, json!(false));

    let expr_not_false = Expression::UnaryOp {
        op: UnaryOperator::Not,
        expr: Box::new(Expression::BoolLiteral(false)),
    };
    let res = evaluate(&expr_not_false, &ctx, None, None).await.unwrap();
    assert_eq!(res, json!(true));

    // Test UnaryOperator::Neg
    let expr_neg = Expression::UnaryOp {
        op: UnaryOperator::Neg,
        expr: Box::new(Expression::Literal(10.0)),
    };
    let res = evaluate(&expr_neg, &ctx, None, None).await.unwrap();
    assert_eq!(res, json!(-10.0));

    // Test UnaryOperator::Not error (type mismatch)
    let expr_not_err = Expression::UnaryOp {
        op: UnaryOperator::Not,
        expr: Box::new(Expression::Literal(5.0)),
    };
    let res = evaluate(&expr_not_err, &ctx, None, None).await;
    assert!(res.is_err());

    // Test UnaryOperator::Neg error (type mismatch)
    let expr_neg_err = Expression::UnaryOp {
        op: UnaryOperator::Neg,
        expr: Box::new(Expression::BoolLiteral(true)),
    };
    let res = evaluate(&expr_neg_err, &ctx, None, None).await;
    assert!(res.is_err());
}
