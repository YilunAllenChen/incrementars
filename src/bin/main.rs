use clap::Parser;
use incrementars::prelude::*;
use incrementars::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // this actually causes stack overflow if number too big... we're using too much stack.
    // but I guess it's not that big of a deal if we don't allocate million of nodes.
    #[arg(short, long, default_value_t = 150000)]
    count: u32,
}

pub fn perf_test() {
    let args = Args::parse();

    let mut dag = Incrementars::new();
    let var = dag.var(0);
    let mut map: Map1<i32, i32> = dag.map(as_input!(var), |x| x + 1);

    for _ in 0..args.count {
        map = dag.map(as_input!(map), |x| x + 1);
    }

    // time it
    let start = std::time::Instant::now();
    var.set(10);
    dag.stablize();
    let end = std::time::Instant::now();
    println!(
        "time: {:?}, throughput: {:.0} k nodes/sec, nanos per node: {:.2}. Final value: {:.2}",
        end - start,
        (args.count as f64) / (end - start).as_secs_f64() / 1_000.0,
        (end - start).as_nanos() / (args.count as u128),
        map.observe()
    );
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
    dag.map2(as_input!(left3), as_input!(right), |x, y| println!("left: {}, right: {}", x, y));

    root.set(20);
    dag.stablize();
}

pub fn main() {
    perf_test()
    // only_run_once()
}
