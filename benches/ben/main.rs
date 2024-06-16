use criterion::{black_box, criterion_group, criterion_main, Criterion};
use incrementars::{
    as_input,
    prelude::{Incrementars, Map1},
};
use std::time::Instant;

fn your_function_to_benchmark(count: i32) {
    let mut dag = Incrementars::new();
    let var = dag.var(0);
    let mut map: Map1<i32, i32> = dag.map(as_input!(var), |x| x + 1);

    for _ in 0..count {
        map = dag.map(as_input!(map), |x| x + 1);
    }
    var.set(10);
    dag.stablize();
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("linear", |b| {
        b.iter(|| {
            let start = Instant::now();
            your_function_to_benchmark(10000);
            black_box(start.elapsed().as_nanos());
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
