use std::collections::VecDeque;
use std::{cell::RefCell, rc::Rc};

use super::bind2::{Bind2, BindHandle};
use super::map::{Map, MapHandle};
use super::map2::{Map2, Map2Handle};
use super::traits::{handles, NodeBehavior, NodeId};
use super::traits::{NodeBehaviorHandle, NodeInputHandle};
use super::var::{Var, VarHandle};

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
