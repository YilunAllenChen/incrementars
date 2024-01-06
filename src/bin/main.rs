use incrementars::*;

fn main() {
    //   v2  v1   ( 1, 1 )  -> ( 1, 5 )
    //    \ /
    // v3  m1     ( 2, 2 )  -> ( 2, 6 )
    //  \ /  \
    //   m2   \   ( 4 )     -> ( 12 )
    //     \  |
    //       m3   ( "6" )   -> ( "18" )

    let mut g = Graph::new();
    let (v1w, v1r) = g.var(1);
    let (_, v2r) = g.var(1);
    let (_, v3r) = g.var(2);
    let (_, m1r) = g.map2(&v1r, &v2r, |a, b| a + b);
    let (_, m2r) = g.map2(&m1r, &v3r, |a, b| a * b);
    let (_, m3r) = g.map2(&m1r, &m2r, |a, b| (a + b).to_string());
    assert_eq!(m3r.borrow().value(), "6".to_string());

    g.stablize();
    v1w.borrow_mut().set(5);

    g.stablize();
    assert_eq!(m3r.borrow().value(), "18".to_string());
}
