use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use incrementars::prelude::{Incrementars, Map1};
use std::time::{Duration, Instant};

fn criterion_benchmark(c: &mut Criterion) {
    let mut cri = c.benchmark_group("custom");

    cri.warm_up_time(Duration::from_millis(50))
        .measurement_time(Duration::from_secs(1));

    cri.bench_function("linear", |b| {
        b.iter(move || {
            let count = 100_000;
            let mut dag = Incrementars::new();
            let var = dag.var(0);
            let mut map: Map1<i32, i32> = dag.map(var.as_input(), |x| x + 1);

            for _ in 0..count {
                map = dag.map(map.as_input(), |x| x + 1);
            }
            let start = Instant::now();
            dag.stablize();
            black_box(start.elapsed().as_nanos());
        })
    });

    for input in [100, 1_000, 10_000, 100_000] {
        cri.bench_with_input(BenchmarkId::from_parameter(input), &input, |b, &i| {
            b.iter(move || {
                let count = i;
                let mut dag = Incrementars::new();
                let var = dag.var(0);
                let mut map: Map1<i32, i32> = dag.map(var.as_input(), |x| x + 1);

                for _ in 0..count {
                    map = dag.map(map.as_input(), |x| x + 1);
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
