use crate::node::*;

pub fn _example() {
    let mut incrementars = Incrementars::new();
    let var = incrementars.var(0);
    var.set(10);
    let map = incrementars.map(Box::new(var), |x| x + 1);
    assert_eq!(map.observe(), 10);
}
