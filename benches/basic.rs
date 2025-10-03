use criterion::{criterion_group, criterion_main, Criterion, black_box};
use std::time::Duration;

pub fn bench_json_parsing(c: &mut Criterion) {
    let sample = r#"{\"command\":\"write\",\"params\":{\"data\":\"ATZ\\r\\n\"}}"#;
    c.bench_function("parse_write_command", |b| {
        b.iter(|| {
            let v: serde_json::Value = serde_json::from_str(sample).unwrap();
            black_box(v);
        })
    });
}

criterion_group!{
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(300))
        .measurement_time(Duration::from_secs(2));
    targets = bench_json_parsing
}
criterion_main!(benches);
