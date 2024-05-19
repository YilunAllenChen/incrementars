use crate::{as_input, node::*};

pub fn _example() {
    let mut compute = Incrementars::new();
    let var = compute.var(0);
    let map = compute.map(as_input!(var), |x| x + 1);
    let map2 = compute.map(as_input!(var), |x| x + 1);
    assert_eq!(map.observe(), 1);
    assert_eq!(map2.observe(), 1);

    var.set(10);
    assert_eq!(map.observe(), 1);

    compute.stablize();
    assert_eq!(map.observe(), 11);

    let map3 = compute.map2(as_input!(var), as_input!(var), |x, y| x + y);
    assert_eq!(map3.observe(), 2);
}
