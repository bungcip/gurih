use gurih_dsl::parser::parse;
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_parser() {
    // Generate a large KDL string
    let count = 10000;
    let mut kdl = String::new();
    for i in 0..count {
        kdl.push_str(&format!("entity \"Entity{}\" {{ belongs_to \"TargetEntity\" }}\n", i));
    }

    // Warmup
    for _ in 0..10 {
        let _ = parse(&kdl, None).unwrap();
    }

    // Benchmark
    let iterations = 100;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = parse(&kdl, None).unwrap();
    }
    let duration = start.elapsed();
    let avg_time = duration / iterations;

    println!("Benchmark result: {:?} per iteration (avg over {} iterations)", avg_time, iterations);
}
