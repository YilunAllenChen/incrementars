use incrementars::Graph;
use incrementars::Node;

fn main() {
    let mut g = Graph::new();
    let v1 = g.var(2);
    let v2 = g.var(3);
    let v3 = g.var(5);
    let m1 = g.map2(&v1, &v2, |a, b| a + b);
    let m2 = g.map2(&m1, &v3, |a, b| a * b);
    assert_eq!(m2.borrow().value(), 25);

    v1.borrow_mut().set(4);
    g.stablize();
    assert_eq!(m2.borrow().value(), 35);
}
