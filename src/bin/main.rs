use std::time::Duration;

use incrementars::prelude::*;

const LINEAR_COUNT: u32 = 150_000;
const EXPAND_NODES: u32 = 150_000;
const JOIN_NODES: u32 = 100_000;

struct Metrics {
    name: &'static str,
    num_node: u32,
    total_time_ms: f64,
    per_node: f64,
}

impl Metrics {
    fn display(&self) {
        println!(
            "{0: <10} | {1: <7}nodes | total {2: <4}ms | {3: <4}ns/node",
            self.name,
            self.num_node,
            self.total_time_ms.round(),
            self.per_node
        )
    }
}

fn linear() -> Metrics {
    let count = LINEAR_COUNT;
    let mut dag = Incrementars::new();
    let var = dag.var(0);
    let mut map: Map1<i32, i32> = dag.map(var.as_input(), |x| x + 1);
    for _ in 0..count {
        map = dag.map(map.as_input(), |x| x + 1);
    }
    let start = std::time::Instant::now();
    var.set(10);
    dag.stabilize();
    let elapsed = start.elapsed().as_secs_f64();
    Metrics {
        name: "linear",
        num_node: count,
        total_time_ms: elapsed * 1e3,
        per_node: (elapsed / count as f64 * 1e9).round(),
    }
}

fn expand() -> Metrics {
    let layers = EXPAND_NODES;
    let mut count = 0;
    let mut dag = Incrementars::new();
    let var = dag.var(0);
    let map0 = dag.map(var.as_input(), |x| x + 1);
    let mut queue: Vec<Box<Map1<i32, i32>>> = vec![map0.as_input()];
    for _ in 0..layers / 2 {
        let head = queue.pop().unwrap();
        let out1 = dag.map(head.clone(), |x| x + 1);
        let out2 = dag.map(head, |x| x + 2);
        queue.push(out1.as_input());
        queue.push(out2.as_input());
        count += 2;
    }
    let start = std::time::Instant::now();
    var.set(10);
    dag.stabilize();
    let elapsed = start.elapsed().as_secs_f64();
    Metrics {
        name: "expand",
        num_node: count,
        total_time_ms: elapsed * 1e3,
        per_node: (elapsed / count as f64 * 1e9).round(),
    }
}

fn join() -> Metrics {
    let vars_num = JOIN_NODES / 2;
    let mut count = vars_num;
    let mut dag = Incrementars::new();

    let vars = (0..vars_num).map(|i| dag.var(i)).collect::<Vec<_>>();
    let mut queue = vars
        .chunks(2)
        .filter_map(|chunk| match chunk {
            [a, b] => Some(dag.map2(a.as_input(), b.as_input(), |x, y| x + y)),
            _ => None,
        })
        .collect::<Vec<_>>();

    count += queue.len() as u32;

    while queue.len() > 2 {
        let in1 = queue.pop().unwrap();
        if let Some(in2) = queue.pop() {
            queue.push(dag.map2(in1.as_input(), in2.as_input(), |x, y| x + y));
            count += 2;
        }
    }

    vars.iter().for_each(|n| n.set(n.observe() + 1));

    std::thread::sleep(Duration::from_secs(1));
    let start = std::time::Instant::now();
    dag.stabilize();
    let elapsed = start.elapsed().as_secs_f64();
    Metrics {
        name: "join",
        num_node: count,
        total_time_ms: elapsed * 1e3,
        per_node: (elapsed * 1e9 / count as f64).round(),
    }
}

fn iter() -> Metrics {
    let layers = 1_000;
    let iterations = 30_000;
    let mut count = 0;
    let mut dag = Incrementars::new();
    let var = dag.var(0);
    let map0 = dag.map(var.as_input(), |x| x);
    let mut queue: Vec<Box<Map1<i32, i32>>> = vec![map0.as_input()];
    for _ in 0..layers / 2 {
        let head = queue.pop().unwrap();
        let out1 = dag.map(head.clone(), |x| x);
        let out2 = dag.map(head, |x| x);
        queue.push(out1.as_input());
        queue.push(out2.as_input());
        count += 2;
    }
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        var.set(10);
        dag.stabilize();
    }
    let elapsed = start.elapsed().as_secs_f64();
    Metrics {
        name: "iter",
        num_node: count,
        total_time_ms: elapsed * 1e3 / iterations as f64,
        per_node: (elapsed / count as f64 * 1e9 / iterations as f64).round(),
    }
}

fn main() {
    for metrics in [linear(), expand(), join(), iter()] {
        metrics.display();
    }
}
