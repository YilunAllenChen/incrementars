use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use incrementars::{
    as_input,
    prelude::{Incrementars, Map1},
};
use std::time::Instant;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("linear", |b| {
        b.iter(move || {
            let count = 100_000;
            let mut dag = Incrementars::new();
            let var = dag.var(0);
            let mut map: Map1<i32, i32> = dag.map(as_input!(var), |x| x + 1);

            for _ in 0..count {
                map = dag.map(as_input!(map), |x| x + 1);
            }
            let start = Instant::now();
            dag.stablize();
            black_box(start.elapsed().as_nanos());
        })
    });

    let mut group = c.benchmark_group("linear with params");
    for input in [100, 1_000, 10_000, 100_000] {
        group.bench_with_input(BenchmarkId::from_parameter(input), &input, |b, &i| {
            b.iter(move || {
                let count = i;
                let mut dag = Incrementars::new();
                let var = dag.var(0);
                let mut map: Map1<i32, i32> = dag.map(as_input!(var), |x| x + 1);

                for _ in 0..count {
                    map = dag.map(as_input!(map), |x| x + 1);
                }
                let start = Instant::now();
                dag.stablize();
                black_box(start.elapsed().as_nanos());
            });
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
