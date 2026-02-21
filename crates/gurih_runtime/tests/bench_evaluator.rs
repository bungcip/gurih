use gurih_ir::{BinaryOperator, Expression};
use gurih_runtime::evaluator::evaluate;
use serde_json::json;
use std::time::Instant;

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
async fn bench_evaluate_deep() {
    // 2000 depth is enough to show recursion overhead
    // If stack overflow happens, we know recursion is too deep for stack, but async recursion uses heap (Box::pin).
    // So 2000 should be fine on heap.
    let depth = 2000;
    let expr = build_deep_expression(depth);
    let ctx = json!({});

    // Warmup
    for _ in 0..10 {
        let _ = evaluate(&expr, &ctx, None, None).await.unwrap();
    }

    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = evaluate(&expr, &ctx, None, None).await.unwrap();
    }
    let duration = start.elapsed();

    println!(
        "BENCHMARK_RESULT: Depth={}, Iterations={}, TotalTime={:?}, AvgTime={:?}",
        depth,
        iterations,
        duration,
        duration / iterations as u32
    );
}
