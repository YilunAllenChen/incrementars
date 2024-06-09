use clap::Parser;
use incrementars::prelude::*;
use incrementars::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // this actually causes stack overflow if number too big... we're using too much stack.
    // but I guess it's not that big of a deal if we don't allocate million of nodes.
    #[arg(short, long, default_value_t = 10000)]
    count: u32,
}

pub fn main() {
    let args = Args::parse();

    let mut compute = Incrementars::new();
    let var = compute.var(0);
    let mut map: Map1<i32, i32> = compute.map(as_input!(var), |x| x + 1);

    for _ in 0..args.count {
        map = compute.map(as_input!(map), |x| x + 1);
    }

    // time it
    let start = std::time::Instant::now();
    var.set(10);
    compute.stablize();
    let end = std::time::Instant::now();
    println!(
        "time: {:?}, throughput: {:.0} k nodes/sec, nanos per node: {:.2}. Final value: {:.2}",
        end - start,
        (args.count as f64) / (end - start).as_secs_f64() / 1_000.0,
        (end - start).as_nanos() / (args.count as u128),
        map.observe()
    );
}
