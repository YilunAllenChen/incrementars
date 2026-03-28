use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use incrementars::prelude::{Incrementars, Map1, Observable, Var};
use std::time::Duration;

fn build_linear(count: usize) -> (Incrementars, Var<i32>) {
    let mut dag = Incrementars::new();
    let input = dag.var(0);
    let mut map: Map1<i32, i32> = dag.map(input.clone(), |x| x + 1);
    for _ in 0..count {
        map = dag.map(map, |x| x + 1);
    }
    (dag, input)
}

fn build_fanout(branches: usize) -> (Incrementars, Var<i32>) {
    let mut dag = Incrementars::new();
    let input = dag.var(0);
    for offset in 0..branches {
        dag.map(input.clone(), move |x| x + offset as i32);
    }
    (dag, input)
}

fn build_join(width: usize) -> (Incrementars, Vec<Var<i32>>) {
    let mut dag = Incrementars::new();
    let vars = (0..width as i32).map(|i| dag.var(i)).collect::<Vec<_>>();
    dag.mapn(
        vars.iter()
            .cloned()
            .map(|var| Box::new(var) as Box<dyn Observable<i32>>)
            .collect(),
        |values| values.into_iter().sum::<i32>(),
    );
    (dag, vars)
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("incrementars");
    group
        .warm_up_time(Duration::from_millis(100))
        .measurement_time(Duration::from_secs(2));

    group.bench_function("linear_build_100k", |b| {
        b.iter(|| {
            let (dag, _) = build_linear(100_000);
            black_box(dag);
        })
    });

    for size in [100usize, 1_000, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("linear_stabilize", size),
            &size,
            |b, &size| {
                let (mut dag, input) = build_linear(size);
                b.iter(|| {
                    input.set(black_box(1));
                    dag.stabilize();
                    input.set(black_box(0));
                    dag.stabilize();
                });
            },
        );
    }

    group.bench_with_input(
        BenchmarkId::new("fanout_stabilize", 50_000),
        &50_000usize,
        |b, &branches| {
            let (mut dag, input) = build_fanout(branches);
            b.iter(|| {
                input.set(black_box(1));
                dag.stabilize();
                input.set(black_box(0));
                dag.stabilize();
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("join_stabilize", 10_000),
        &10_000usize,
        |b, &width| {
            let (mut dag, vars) = build_join(width);
            b.iter(|| {
                for (index, var) in vars.iter().enumerate() {
                    var.set(black_box(index as i32 + 1));
                }
                dag.stabilize();
            });
        },
    );

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
