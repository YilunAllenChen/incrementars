use std::time::Duration;

use clap::Parser;
use incrementars::prelude::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // this actually causes stack overflow if number too big... we're using too much stack.
    // but I guess it's not that big of a deal if we don't allocate million of nodes.
    #[arg(short, long, default_value_t = 150000)]
    linear_count: u32,

    #[arg(short, long, default_value_t = 150000)]
    expand_nodes: u32,

    #[arg(short, long, default_value_t = 100000)]
    join_nodes: u32,
}

pub fn perf_test() {
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
    let args = Args::parse();

    fn linear(args: &Args) -> Metrics {
        let count = args.linear_count;
        let mut dag = Incrementars::new();
        let var = dag.var(0);
        let mut map: Map1<i32, i32> = dag.map(var.as_input(), |x| x + 1);

        for _ in 0..count {
            map = dag.map(map.as_input(), |x| x + 1);
        }

        // time it
        let start = std::time::Instant::now();
        var.set(10);
        dag.stablize();
        let end = std::time::Instant::now();
        Metrics {
            name: "linear",
            num_node: count,
            total_time_ms: (end - start).as_secs_f64() * 1e3,
            per_node: ((end - start).as_secs_f64() / count as f64 * 1e9).round(),
        }
    }

    fn expand(args: &Args) -> Metrics {
        let layers = args.expand_nodes;
        let mut count = 0;
        let mut dag = Incrementars::new();
        let var = dag.var(0);
        let map0 = dag.map(var.as_input(), |x| x + 1);
        let mut queues: Vec<Box<Map1<i32, i32>>> = vec![map0.as_input()];
        for _ in 0..layers / 2 {
            let head = queues.pop().unwrap();
            let out1 = dag.map(head.clone(), |x| x + 1);
            let out2 = dag.map(head, |x| x + 2);
            queues.push(out1.as_input());
            queues.push(out2.as_input());
            count += 2;
        }

        let start = std::time::Instant::now();
        var.set(10);
        dag.stablize();
        let end = std::time::Instant::now();
        Metrics {
            name: "expand",
            num_node: count,
            total_time_ms: (end - start).as_secs_f64() * 1e3,
            per_node: ((end - start).as_secs_f64() / count as f64 * 1e9).round(),
        }
    }

    fn join(args: &Args) -> Metrics {
        let vars_num = args.join_nodes / 2;
        let mut count = vars_num;
        let mut dag = Incrementars::new();

        let vars = (0..vars_num)
            .into_iter()
            .map(|i| dag.var(i))
            .collect::<Vec<_>>();
        let mut queue = vars
            .chunks(2)
            .filter_map(|vars| match vars.len() {
                2 => Some(dag.map2(vars[0].as_input(), vars[1].as_input(), |x, y| x + y)),
                1 => None,
                _ => panic!("wtf?"),
            })
            .collect::<Vec<_>>();

        count += queue.len() as u32;

        while queue.len() > 2 {
            let in1 = queue.pop().unwrap();
            let in2 = queue.pop();
            match in2 {
                Some(in2) => {
                    queue.push(dag.map2(in1.as_input(), in2.as_input(), |x, y| x + y));
                    count += 2;
                }
                None => continue,
            }
        }

        vars.into_iter().for_each(|n| n.set(n.observe() + 1));

        std::thread::sleep(Duration::from_secs(1));
        let start = std::time::Instant::now();
        dag.stablize();
        let end = std::time::Instant::now();
        Metrics {
            name: "join",
            num_node: count,
            total_time_ms: (end - start).as_secs_f64() * 1e3,
            per_node: ((end - start).as_secs_f64() * 1e9 / count as f64).round(),
        }
    }

    fn iter(_args: &Args) -> Metrics {
        let layers = 1_000;
        let iter = 30_000;
        let mut count = 0;

        let mut dag = Incrementars::new();
        let var = dag.var(0);
        let map0 = dag.map(var.as_input(), |x| x);
        let mut queues: Vec<Box<Map1<i32, i32>>> = vec![map0.as_input()];
        for _ in 0..layers / 2 {
            let head = queues.pop().unwrap();
            let out1 = dag.map(head.clone(), |x| x);
            let out2 = dag.map(head, |x| x);
            queues.push(out1.as_input());
            queues.push(out2.as_input());
            count += 2;
        }
        let start = std::time::Instant::now();
        for _ in 0..iter {
            var.set(10);
            dag.stablize();
        }
        let end = std::time::Instant::now();
        Metrics {
            name: "expand",
            num_node: count,
            total_time_ms: (end - start).as_secs_f64() * 1e3 / iter as f64,
            per_node: ((end - start).as_secs_f64() / count as f64 * 1e9 / iter as f64).round(),
        }
    }

    vec![linear, expand, join, iter]
        .into_iter()
        .for_each(|fun| fun(&args).display());
}

pub fn main() {
    perf_test()
}
