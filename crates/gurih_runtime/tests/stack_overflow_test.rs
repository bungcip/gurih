#[cfg(test)]
mod tests {
    use gurih_runtime::evaluator::evaluate;
    use gurih_ir::{BinaryOperator, Expression};
    use serde_json::json;

    #[tokio::test]
    async fn test_like_operator_stack_overflow() {
        // Create a pattern with many wildcards to trigger deep recursion
        let mut pattern = String::new();
        for _ in 0..10000 {
            pattern.push('%');
        }
        pattern.push('a');

        let mut text = String::new();
        for _ in 0..10000 {
            text.push('a');
        }

        let expr = Expression::BinaryOp {
            left: Box::new(Expression::StringLiteral(text)),
            op: BinaryOperator::Like,
            right: Box::new(Expression::StringLiteral(pattern)),
        };

        let ctx = json!({});
        // This should pass with iterative implementation, but would crash with recursive
        let res = evaluate(&expr, &ctx, None, None).await;
        assert!(res.is_ok());
    }
}
