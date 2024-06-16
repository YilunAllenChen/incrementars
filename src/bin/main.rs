use std::ops::Deref;

use incrementars::{map1, peek, stablize, var};

pub fn main() {
    let count = 150000;
    let var = var(0);
    let mut map = map1(var.clone(), |x| x + 1);
    for _ in 0..count {
        map = map1(map, |x| x + 1)
    }
    println!("{}", peek(map.clone()));

    var.deref().borrow_mut().set(0);
    let start = std::time::Instant::now();
    stablize(map.clone());
    let end = std::time::Instant::now();
    println!(
        "time: {:?}, throughput: {:.0} k nodes/sec, nanos per node: {:.2}. Final value: {:.2}",
        end - start,
        (count as f64) / (end - start).as_secs_f64() / 1_000.0,
        (end - start).as_nanos() / (count as u128),
        peek(map)
    );
}
