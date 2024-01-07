#[cfg(test)]
mod tests {

    use crate::prelude::*;

    #[test]
    fn test_graph() {
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

    #[test]
    fn test_tree() {
        //  v1  v2  v3  v4
        //    \ /    \ /
        //     m1    m2
        //       \  /
        //        m3

        let mut g = Graph::new();
        let (v1w, v1r) = g.var(1);
        let (_, v2r) = g.var(1);
        let (_, v3r) = g.var(1);
        let (_, v4r) = g.var(1);
        let (_, m1r) = g.map2(&v1r, &v2r, |a, b| a + b);
        let (_, m2r) = g.map2(&m1r, &v3r, |a, b| a + b);
        let (_, m3r) = g.map2(&m2r, &v4r, |a, b| a + b);
        assert_eq!(m3r.borrow().value(), 4);

        g.stablize();
        v1w.borrow_mut().set(5);
        g.stablize();
        assert_eq!(m3r.borrow().value(), 8);
    }

    #[test]
    fn test_pipeline() {
        // v1       v2
        //  |        |
        // m11      m21
        //  |        |
        // m12  v3  m22
        //  |  /  \  |
        // m13      m23
        //  |        |
        // o1       o2

        let incr = |x| x + 1;
        let mut g = Graph::new();

        let (_, v1r) = g.var(1);
        let (v2w, v2r) = g.var(2);
        let (_, v3r) = g.var(10);

        let (_, m11r) = g.map(&v1r, incr);
        let (_, m12r) = g.map(&m11r, incr);
        let (_, m13r) = g.map2(&m12r, &v3r, |a, b| a + b);
        let (_, o1) = g.map(&m13r, |a| a.to_string());

        let (_, m21r) = g.map(&v2r, incr);
        let (_, m22r) = g.map(&m21r, incr);
        let (_, m23r) = g.map2(&m22r, &v3r, |a, b| a + b);
        let (_, o2) = g.map(&m23r, |a| a.to_string());

        g.stablize();
        assert_eq!(o1.borrow().value(), "13".to_string());
        assert_eq!(o2.borrow().value(), "14".to_string());

        v2w.borrow_mut().set(5);
        assert_eq!(o1.borrow().value(), "13".to_string());
        g.stablize();
        assert_eq!(o1.borrow().value(), "13".to_string());
        assert_eq!(o2.borrow().value(), "17".to_string());
    }

    #[test]
    fn test_bind() {
        //      v2                       v2
        //      |                        |
        // v1   m1                  v1   m1
        //   \                           /
        //    b1 <- c1   becomes       b1 <- c1
        //    |                        |
        //    o                        o

        let mut g = Graph::new();
        let (v1w, v1r) = g.var(1);
        let (v2w, v2r) = g.var(2);
        let (_, m1r) = g.map(&v2r, |a| a + 1);

        #[derive(Clone)]
        enum Ctrl {
            V1,
            M1,
        }

        let (c1w, c1r) = g.var(Ctrl::V1);

        let (_, b1r) = g.bind2(
            &c1r,
            move |c, vec| match c {
                Ctrl::V1 => vec.0,
                Ctrl::M1 => vec.1,
            },
            (v1r, m1r),
        );

        let (_, o) = g.map(&b1r, |a| a.to_string());

        g.stablize();
        assert_eq!(o.borrow().value(), "1".to_string());

        c1w.borrow_mut().set(Ctrl::M1);
        g.stablize();
        assert_eq!(o.borrow().value(), "3".to_string());

        // a change on v2 should propagte all the way.
        v2w.borrow_mut().set(5);
        g.stablize();
        assert_eq!(o.borrow().value(), "6".to_string());

        // a change on v1 should not propagate because nothing is bound to it.
        // b1 still points to m1.
        v1w.borrow_mut().set(10);
        g.stablize();
        assert_eq!(o.borrow().value(), "6".to_string());
    }
}
