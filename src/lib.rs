use std::{cell::RefCell, rc::Rc};
mod node;

use node::map2::{Map2, Map2Handle};
use node::var::{Var, VarHandle};
use node::traits::NodeInputHandle;
pub use node::traits::{handles, NodeBehavior, NodeId, NodeInput, NodeValue};

pub struct Graph {
    nodes: Vec<Rc<RefCell<dyn NodeBehavior>>>,
    pub id_counter: u64,
    node_to_children: Vec<Vec<NodeId>>,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            nodes: vec![],
            id_counter: 0,
            node_to_children: vec![],
        }
    }

    fn get_children(&self, id: NodeId) -> Vec<Rc<RefCell<dyn NodeBehavior>>> {
        match self.node_to_children.get(id as usize) {
            Some(children) => children
                .iter()
                .map(|i| self.nodes[*i as usize].clone())
                .collect(),
            None => panic!("Node not found {id}"),
        }
    }

    pub fn gen_id(&mut self) -> NodeId {
        let num = self.id_counter;
        self.id_counter += 1;
        num
    }

    pub fn var<T: Clone + 'static>(&mut self, value: T) -> (VarHandle<T>, NodeInputHandle<T>) {
        let node_id = self.gen_id();
        let node = Var::make(node_id, value);
        let (ident, input, behavior) = handles(node.clone());
        self.nodes.push(behavior);
        self.node_to_children.push(vec![]);
        (ident, input)
    }

    pub fn map2<In1, In2, SelfT>(
        &mut self,
        n1: &Rc<RefCell<dyn NodeInput<In1>>>,
        n2: &Rc<RefCell<dyn NodeInput<In2>>>,
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
        self.node_to_children.push(vec![]);
        self.node_to_children
            .get_mut(n1.borrow().id() as usize)
            .unwrap()
            .push(node_id);
        self.node_to_children
            .get_mut(n2.borrow().id() as usize)
            .unwrap()
            .push(node_id);
        (ident, input)
    }

    pub fn stablize(&mut self) {
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(self.nodes[0].clone());
        println!("self nodes to children {:?}", self.node_to_children);
        while let Some(node) = queue.pop_front() {
            node.borrow_mut().stablize();
            println!("Stablizing node {}", node.borrow().id());
            for child in self.get_children(node.borrow().id()) {
                println!("should stablize node {}", child.borrow().id());
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
}
