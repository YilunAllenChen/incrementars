use std::collections::VecDeque;
use std::{cell::RefCell, rc::Rc};
mod node;

use node::bind2::{Bind2, BindHandle};
use node::map::{Map, MapHandle};
use node::map2::{Map2, Map2Handle};
pub use node::traits::{handles, NodeBehavior, NodeId, NodeInput, NodeValue};
use node::traits::{NodeBehaviorHandle, NodeInputHandle};
use node::var::{Var, VarHandle};

/**
High level construct to manage nodes and their dependency relationships
*/
pub struct Graph {
    nodes: Vec<Rc<RefCell<dyn NodeBehavior>>>,
    pub id_counter: u64,
    node_to_children_id: Vec<Vec<NodeId>>,
}

impl Graph {
    /**
    Create a new graph
    */
    pub fn new() -> Self {
        Graph {
            nodes: vec![],
            id_counter: 0,
            node_to_children_id: vec![],
        }
    }

    /**
    Get the children of a node in their behavioral form, agnostic to the kind of data they hold.
    Used internally by the graph to propagate dirty changes.
    */
    fn get_children(&self, id: NodeId) -> Vec<NodeBehaviorHandle> {
        match self.node_to_children_id.get(id as usize) {
            Some(children) => children
                .iter()
                .map(|i| self.nodes[*i as usize].clone())
                .collect(),
            None => panic!("Node not found {id}"),
        }
    }

    /**
    Create unique node id for each new node registered under the graph
    */
    fn gen_id(&mut self) -> NodeId {
        let num = self.id_counter;
        self.id_counter += 1;
        num
    }

    /**
     * Create a variable node, and return its setter and reader handles
     */
    pub fn var<T: Clone + 'static>(&mut self, value: T) -> (VarHandle<T>, NodeInputHandle<T>) {
        let node_id = self.gen_id();
        let node = Var::make(node_id, value);
        let (ident, input, behavior) = handles(node.clone());
        self.nodes.push(behavior);
        self.node_to_children_id.push(vec![]);
        (ident, input)
    }

    /**
     * Create a map node (fn 'In -> 'SelfT), and return its setter and reader handles
     */
    pub fn map<In, SelfT>(
        &mut self,
        n: &NodeInputHandle<In>,
        func: fn(In) -> SelfT,
    ) -> (MapHandle<SelfT, In>, NodeInputHandle<SelfT>)
    where
        In: Clone + 'static,
        SelfT: Clone + 'static,
    {
        let node_id = self.gen_id();
        let node = Map::make(node_id, n, func);
        let (ident, input, behavior) = handles(node);
        self.nodes.push(behavior);
        self.node_to_children_id.push(vec![]);
        self.node_to_children_id
            .get_mut(n.borrow().id() as usize)
            .unwrap()
            .push(node_id);
        (ident, input)
    }

    /**
     * Create a map2 node (fn 'In -> 'In2 -> 'SelfT), and return its setter and reader handles
     */
    pub fn map2<In1, In2, SelfT>(
        &mut self,
        n1: &NodeInputHandle<In1>,
        n2: &NodeInputHandle<In2>,
        func: fn(In1, In2) -> SelfT,
    ) -> (Map2Handle<SelfT, In1, In2>, NodeInputHandle<SelfT>)
    where
        In1: Clone + 'static,
        In2: Clone + 'static,
        SelfT: Clone + 'static,
    {
        let node_id = self.gen_id();
        let node = Map2::make(node_id, n1, n2, func);
        let (ident, input, behavior) = handles(node);
        self.nodes.push(behavior);
        self.node_to_children_id.push(vec![]);
        self.node_to_children_id
            .get_mut(n1.borrow().id() as usize)
            .unwrap()
            .push(node_id);
        self.node_to_children_id
            .get_mut(n2.borrow().id() as usize)
            .unwrap()
            .push(node_id);
        (ident, input)
    }

    /**
     * Create a bind2 node (fn 'In -> '[Node<'SelfT>], -> 'Node<'SelfT>), and return its setter and reader handles
     * This is useful when one wants to create dynamism in node topology. In a complex graph where you want to avoid
     * refiring graph when there is a change to (temporarily) unused state, you can use bind to create a node that
     * dynamically choose which node to listen to depending on the input.
     */
    pub fn bind2<In, SelfT>(
        &mut self,
        n: &NodeInputHandle<In>,
        func: fn(In, (NodeInputHandle<SelfT>, NodeInputHandle<SelfT>)) -> NodeInputHandle<SelfT>,
        captured_inputs: (NodeInputHandle<SelfT>, NodeInputHandle<SelfT>),
    ) -> (BindHandle<SelfT, In>, NodeInputHandle<SelfT>)
    where
        In: Clone + 'static,
        SelfT: Clone + 'static,
    {
        let node_id = self.gen_id();
        let node = Bind2::make(node_id, n, func, captured_inputs.clone());
        let (ident, input, behavior) = handles(node);
        self.nodes.push(behavior);
        self.node_to_children_id.push(vec![]);
        self.node_to_children_id
            .get_mut(n.borrow().id() as usize)
            .unwrap()
            .push(node_id);
        [&captured_inputs.0, &captured_inputs.1]
            .into_iter()
            .for_each(|i| {
                self.node_to_children_id
                    .get_mut(i.borrow().id() as usize)
                    .unwrap()
                    .push(node_id);
            });
        (ident, input)
    }

    /**
    Recompute all the necessary nodes in the graph.
    */
    pub fn stablize(&mut self) {
        let mut queue = VecDeque::new();
        self.nodes
            .iter()
            .filter_map(|node| {
                if node.borrow().dirty() {
                    Some(node.clone())
                } else {
                    None
                }
            })
            .for_each(|node| {
                queue.push_back(node);
            });
        while let Some(node) = queue.pop_front() {
            node.borrow_mut().stablize();
            for child in self.get_children(node.borrow().id()) {
                queue.push_back(child);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

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
