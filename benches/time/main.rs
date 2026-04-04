use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use incrementars::prelude::{Graph, IntoInput, Map1, Signal, Var};
use std::time::{Duration, Instant};

#[allow(dead_code)]
fn raw_linear(count: usize, start: i32) -> i32 {
    let mut x = start;
    for _ in 0..=count {
        x += 1;
    }
    x
}

fn build_linear(count: usize) -> (Graph, Var<i32>) {
    let mut dag = Graph::new();
    let input = dag.var(0);
    let mut map: Map1<i32, i32> = dag.map(input.clone(), |x| x + 1);
    for _ in 0..count {
        map = dag.map(map, |x| x + 1);
    }
    (dag, input)
}

fn build_fanout(branches: usize) -> (Graph, Var<i32>) {
    let mut dag = Graph::new();
    let input = dag.var(0);
    for offset in 0..branches {
        dag.map(input.clone(), move |x| x + offset as i32);
    }
    (dag, input)
}

fn build_expand(layers: usize) -> (Graph, Var<i32>) {
    let mut dag = Graph::new();
    let input = dag.var(0);
    let root: Map1<i32, i32> = dag.map(input.clone(), |x| x + 1);
    let mut queue = vec![root];
    for _ in 0..layers / 2 {
        let head = queue.pop().unwrap();
        let out1 = dag.map(head.clone(), |x| x + 1);
        let out2 = dag.map(head, |x| x + 2);
        queue.push(out1);
        queue.push(out2);
    }
    (dag, input)
}

fn build_iter_tree(layers: usize) -> (Graph, Var<i32>) {
    let mut dag = Graph::new();
    let input = dag.var(0);
    let root: Map1<i32, i32> = dag.map(input.clone(), |x| x);
    let mut queue = vec![root];
    for _ in 0..layers / 2 {
        let head = queue.pop().unwrap();
        let out1 = dag.map(head.clone(), |x| x);
        let out2 = dag.map(head, |x| x);
        queue.push(out1);
        queue.push(out2);
    }
    (dag, input)
}

fn build_join(width: usize) -> (Graph, Vec<Var<i32>>) {
    let mut dag = Graph::new();
    let vars = (0..width as i32).map(|i| dag.var(i)).collect::<Vec<_>>();
    dag.mapn(vars.iter(), |values| values.into_iter().sum::<i32>());
    (dag, vars)
}

/// Builds a layered DAG: `depth` layers × `width` nodes each.
///
/// Node mix per layer (deterministic, based on position):
///   50% map    — x + 1
///   25% map2   — a + b
///  12.5% map3  — (a + b + c) / 3
///  12.5% bind  — selector fixed at 0, always picks one of two parents; no rewires
///
/// Returns the graph and the input Var handles (one per node in layer 0).
/// A second set of selector Vars drives the bind nodes but is never dirtied
/// during the benchmark, so bind stabilization follows the normal Changed/Unchanged
/// path rather than the Rebound path.
fn build_realistic(depth: usize, width: usize) -> (Graph, Vec<Var<i32>>) {
    let mut dag = Graph::new();

    let input_vars: Vec<Var<i32>> = (0..width).map(|i| dag.var(i as i32)).collect();
    // Selector vars for bind: held at 0 throughout the benchmark.
    let selector_vars: Vec<Var<i32>> = (0..width).map(|_| dag.var(0i32)).collect();

    let mut layer: Vec<Signal<i32>> = input_vars.iter().map(|v| v.clone().into_input()).collect();
    let sel_layer: Vec<Signal<i32>> = selector_vars.iter().map(|v| v.clone().into_input()).collect();

    for d in 1..depth {
        let mut next: Vec<Signal<i32>> = Vec::with_capacity(width);
        for i in 0..width {
            // 0-7 → map (50%), 8-11 → map2 (25%), 12-13 → map3 (12.5%), 14-15 → bind (12.5%)
            let kind = (d.wrapping_mul(97).wrapping_add(i.wrapping_mul(31))) % 16;
            let p = |off: usize| layer[(i + off) % width].clone();
            let incr: Signal<i32> = match kind {
                0..=7 => dag.map(p(0), |x| x.wrapping_add(1)).into_input(),
                8..=11 => dag.map2(p(0), p(3), |a, b| a.wrapping_add(b)).into_input(),
                12..=13 => {
                    dag.map3(p(0), p(3), p(7), |a, b, c| a.wrapping_add(b).wrapping_add(c) / 3)
                        .into_input()
                }
                _ => {
                    // bind: selector is always 0 → always selects p(3), never p(7)
                    let sel = sel_layer[i % width].clone();
                    let p_even = p(3);
                    let p_odd = p(7);
                    dag.bind(sel, move |v| {
                        if v % 2 == 0 {
                            p_even.clone()
                        } else {
                            p_odd.clone()
                        }
                    })
                    .into_input()
                }
            };
            next.push(incr);
        }
        layer = next;
    }

    (dag, input_vars)
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
                let mut val = 0i32;
                b.iter_custom(|iters| {
                    let mut total = Duration::ZERO;
                    for _ in 0..iters {
                        // Set outside the timed region so we measure only stabilize().
                        val = 1 - val;
                        input.set(black_box(val));
                        let t = Instant::now();
                        dag.stabilize();
                        total += t.elapsed();
                    }
                    total
                });
            },
        );
    }

    group.bench_with_input(
        BenchmarkId::new("fanout_stabilize", 50_000),
        &50_000usize,
        |b, &branches| {
            let (mut dag, input) = build_fanout(branches);
            let mut val = 0i32;
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    val = 1 - val;
                    input.set(black_box(val));
                    let t = Instant::now();
                    dag.stabilize();
                    total += t.elapsed();
                }
                total
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("expand_stabilize", 150_000),
        &150_000usize,
        |b, &layers| {
            let (mut dag, input) = build_expand(layers);
            let mut val = 0i32;
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    val = 1 - val;
                    input.set(black_box(val));
                    let t = Instant::now();
                    dag.stabilize();
                    total += t.elapsed();
                }
                total
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("join_stabilize", 10_000),
        &10_000usize,
        |b, &width| {
            let (mut dag, vars) = build_join(width);
            let mut val = 0i32;
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    val = 1 - val;
                    for var in vars.iter() {
                        var.set(black_box(val));
                    }
                    let t = Instant::now();
                    dag.stabilize();
                    total += t.elapsed();
                }
                total
            });
        },
    );

    // Realistic mixed graph: 100 layers × 100 nodes (10k total), mix of map/map2/map3/bind.
    // All 100 input vars are dirtied before each stabilize, so the full graph recomputes.
    group.bench_function("realistic_stabilize", |b| {
        let (mut dag, inputs) = build_realistic(100, 100);
        let mut val = 0i32;
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                val = val.wrapping_add(1);
                for input in &inputs {
                    input.set(black_box(val));
                }
                let t = Instant::now();
                dag.stabilize();
                total += t.elapsed();
            }
            total
        });
    });

    // Repeated stabilization throughput on a small binary tree (1k layers)
    // using the same 30k stabilize loop as the legacy benchmark.
    group.bench_function("iter_stabilize", |b| {
        let (mut dag, input) = build_iter_tree(1_000);
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                let t = Instant::now();
                for _ in 0..30_000 {
                    input.set(black_box(10));
                    dag.stabilize();
                }
                total += t.elapsed();
            }
            total
        });
    });

    // Baseline: same x+1 computation N times in plain Rust (no framework),
    // one call per iteration to match the stabilize benchmarks above.
    for size in [100usize, 1_000, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("raw_linear", size),
            &size,
            |b, &size| {
                let mut val = 0i32;
                b.iter(|| {
                    val = 1 - val;
                    black_box(raw_linear(size, black_box(val)));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
