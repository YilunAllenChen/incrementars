use std::time::Duration;

use clap::Parser;
use incrementars::prelude::*;
use incrementars::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // this actually causes stack overflow if number too big... we're using too much stack.
    // but I guess it's not that big of a deal if we don't allocate million of nodes.
    #[arg(short, long, default_value_t = 150000)]
    linear_count: u32,

    #[arg(short, long, default_value_t = 150000)]
    expand_nodes: u32,

    #[arg(short, long, default_value_t = 150000)]
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
        let mut map: Map1<i32, i32> = dag.map(as_input!(var), |x| x + 1);

        for _ in 0..count {
            map = dag.map(as_input!(map), |x| x + 1);
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
        let map0 = dag.map(as_input!(var), |x| x + 1);
        let mut queues: Vec<Box<Map1<i32, i32>>> = vec![as_input!(map0)];
        for _ in 0..layers / 2 {
            let head = queues.pop().unwrap();
            let out1 = dag.map(head.clone(), |x| x + 1);
            let out2 = dag.map(head, |x| x + 2);
            queues.push(as_input!(out1));
            queues.push(as_input!(out2));
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
            .map(|vars| dag.map2(as_input!(vars[0]), as_input!(vars[1]), |x, y| x + y))
            .collect::<Vec<_>>();

        count += queue.len() as u32;

        while queue.len() > 2 {
            let in1 = queue.pop().unwrap();
            let in2 = queue.pop().unwrap();
            queue.push(dag.map2(as_input!(in1), as_input!(in2), |x, y| x + y));
            count += 1;
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
            per_node: ((end - start).as_secs_f64() / count as f64 * 1e9).round(),
        }
    }

    vec![linear, expand, join]
        .into_iter()
        .for_each(|fun| fun(&args).display());
}

pub fn example() {
    let mut dag = Incrementars::new();
    let length = dag.var(2.0);
    let area = dag.map(as_input!(length), |x| {
        println!("calculating area");
        x * x
    });

    // on initial stabalization, area is calculated to be 4.
    assert_eq!(area.observe(), 4.0);
    length.set(3.0);

    // right after setting, dag isn't stablized yet.
    assert_eq!(area.observe(), 4.0);

    dag.stablize();
    assert_eq!(area.observe(), 9.0);

    println!("introducing height...");
    let height = dag.var(5.0);
    let volume = dag.map2(as_input!(area), as_input!(height), |x, y| {
        println!("calculating volume");
        x * y
    });

    assert_eq!(volume.observe(), 45.0);

    height.set(10.0);
    dag.stablize();
    assert_eq!(volume.observe(), 90.0);

    length.set(2.0);
    dag.stablize();
    assert_eq!(volume.observe(), 40.0);
}

pub fn only_run_once() {
    let mut dag = Incrementars::new();
    let root = dag.var(10);
    let left1 = dag.map(as_input!(root), |x| x + 1);
    let left2 = dag.map(as_input!(left1), |x| x + 1);
    let left3 = dag.map(as_input!(left2), |x| x + 1);
    let right = dag.map(as_input!(root), |x| x * 2);
    dag.map2(as_input!(left3), as_input!(right), |x, y| {
        println!("left: {}, right: {}", x, y)
    });

    root.set(20);
    dag.stablize();
}

pub fn main() {
    perf_test()
    // only_run_once()
}

