use criterion::{criterion_group, criterion_main, Criterion};
use matrix_runner::runner::execution::run_test_case;
use matrix_runner::runner::config::TestCase;
use std::path::PathBuf;
use tokio::runtime::Runtime;

fn bench_run_test_case(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let case = TestCase {
        name: "bench_test".to_string(),
        features: "".to_string(),
        no_default_features: false,
        command: Some("echo bench".to_string()),
        allow_failure: vec![],
        arch: vec![],
        timeout_secs: Some(10),
        retries: None,
    };
    let project_root = PathBuf::from(".");
    let crate_name = "bench_crate".to_string();

    c.bench_function("run_test_case", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = run_test_case(case.clone(), &project_root, &crate_name).await;
        });
    });
}

criterion_group!(benches, bench_run_test_case);
criterion_main!(benches); 