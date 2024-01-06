use std::{cell::RefCell, rc::Rc};

pub type NodeId = u64;

pub trait NodeBehavior {
    fn id(&self) -> NodeId;
    fn stablize(&mut self);
}

pub trait NodeValue<T> {
    fn value(&self) -> T;
}

pub trait NodeInput<In>: NodeBehavior + NodeValue<In> {}

pub trait NodeShare<In> {
    fn as_input(self) -> Rc<RefCell<dyn NodeInput<In>>>;
    fn as_observable(self) -> Rc<RefCell<dyn NodeValue<In>>>;
}

#[derive(Clone)]
pub struct Var<T>
where
    T: Clone,
{
    pub id: NodeId,
    pub value: RefCell<T>,
}

impl<T> Var<T>
where
    T: Clone,
{
    pub fn set(&mut self, value: T) {
        *self.value.borrow_mut() = value;
    }

    pub fn make(id: NodeId, value: T) -> Var<T> {
        Var {
            id,
            value: RefCell::new(value),
        }
    }
}

fn wrap(value: impl NodeBehavior + 'static) -> Rc<RefCell<dyn NodeBehavior>> {
    Rc::new(RefCell::new(value))
}

impl<SelfT: Clone> NodeBehavior for Var<SelfT> {
    fn id(&self) -> NodeId {
        self.id
    }
    fn stablize(&mut self) {}
}

impl<SelfT: Clone> NodeValue<SelfT> for Var<SelfT> {
    fn value(&self) -> SelfT {
        self.value.borrow().clone()
    }
}

impl<SelfT: Clone> NodeInput<SelfT> for Var<SelfT> {}

impl<SelfT: Clone + 'static> NodeShare<SelfT> for Var<SelfT> {
    fn as_input(self) -> Rc<RefCell<dyn NodeInput<SelfT>>> {
        Rc::new(RefCell::new(self)) as Rc<RefCell<dyn NodeInput<SelfT>>>
    }
    fn as_observable(self) -> Rc<RefCell<dyn NodeValue<SelfT>>> {
        Rc::new(RefCell::new(self))
    }
}

#[derive(Clone)]
pub struct Map2<SelfT, In1, In2>
where
    SelfT: Clone,
    In1: Clone,
    In2: Clone,
{
    pub id: NodeId,
    pub value: SelfT,
    pub parents: (
        Rc<RefCell<dyn NodeInput<In1>>>,
        Rc<RefCell<dyn NodeInput<In2>>>,
    ),
    pub func: fn(In1, In2) -> SelfT,
}

impl<SelfT, In1, In2> NodeBehavior for Map2<SelfT, In1, In2>
where
    SelfT: Clone,
    In1: Clone,
    In2: Clone,
{
    fn id(&self) -> NodeId {
        self.id
    }
    fn stablize(&mut self) {
        self.value = (self.func)(
            self.parents.0.borrow().value(),
            self.parents.1.borrow().value(),
        );
    }
}

impl<SelfT, In1, In2> NodeValue<SelfT> for Map2<SelfT, In1, In2>
where
    SelfT: Clone,
    In1: Clone,
    In2: Clone,
{
    fn value(&self) -> SelfT {
        self.value.clone()
    }
}

impl<SelfT, In1, In2> NodeInput<SelfT> for Map2<SelfT, In1, In2>
where
    SelfT: Clone,
    In1: Clone,
    In2: Clone,
{
}

impl<SelfT, In1, In2> Map2<SelfT, In1, In2>
where
    SelfT: Clone,
    In1: Clone,
    In2: Clone,
{
    pub fn make(
        id: NodeId,
        n1: &Rc<RefCell<dyn NodeInput<In1>>>,
        n2: &Rc<RefCell<dyn NodeInput<In2>>>,
        func: fn(In1, In2) -> SelfT,
    ) -> Map2<SelfT, In1, In2> {
        Map2 {
            id,
            value: (func)(n1.borrow().value(), n2.borrow().value()),
            parents: (n1.clone(), n2.clone()),
            func,
        }
    }
}

impl<SelfT, In1, In2> NodeShare<SelfT> for Map2<SelfT, In1, In2>
where
    SelfT: Clone + 'static,
    In1: Clone + 'static,
    In2: Clone + 'static,
{
    fn as_input(self) -> Rc<RefCell<dyn NodeInput<SelfT>>> {
        Rc::new(RefCell::new(self)) as Rc<RefCell<dyn NodeInput<SelfT>>>
    }
    fn as_observable(self) -> Rc<RefCell<dyn NodeValue<SelfT>>> {
        Rc::new(RefCell::new(self))
    }
}

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

    pub fn var<T: Clone + 'static>(&mut self, value: T) -> Var<T> {
        let node_id = self.gen_id();
        let node = Var::make(node_id, value);
        let wrapped = wrap(node.clone());
        self.nodes.push(wrapped);
        self.node_to_children.push(vec![]);
        node
    }

    pub fn map2<In1, In2, SelfT>(
        &mut self,
        n1: &Rc<RefCell<dyn NodeInput<In1>>>,
        n2: &Rc<RefCell<dyn NodeInput<In2>>>,
        func: fn(In1, In2) -> SelfT,
    ) -> Map2<SelfT, In1, In2>
    where
        In1: Clone + 'static,
        In2: Clone + 'static,
        SelfT: Clone + 'static,
    {
        let node_id = self.gen_id();
        let node = Map2::make(node_id, n1, n2, func);
        let wrapped = wrap(node.clone());
        self.nodes.push(wrapped);
        self.node_to_children.push(vec![]);
        self.node_to_children
            .get_mut(n1.borrow().id() as usize)
            .unwrap()
            .push(node_id);
        self.node_to_children
            .get_mut(n2.borrow().id() as usize)
            .unwrap()
            .push(node_id);

        node
    }

    pub fn stablize(&mut self) {
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(self.nodes[0].clone());
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
        //   v1  v2
        //    \ /
        // v3  m1
        //  \ /
        //   m2

        let mut g = Graph::new();
        let v1 = g.var(2);
        let v1i = v1.as_input();
        let v2 = g.var(3);
        let v3 = g.var(5);
        let m1 = g.map2(&v1i, &v2.as_input(), |a, b| a + b);
        let m2 = g.map2(&m1.as_input(), &v3.as_input(), |a, b| a * b);
        let m3 = g.map2(&v1i, &m2.as_input(), |a, b| (a + b).to_string());
        assert_eq!(m3.value(), "27".to_string());

        g.stablize();
        // assert_eq!(m2.value(), 35);
    }

    #[test]
    fn test_tree() {
        //  v1  v2  v3  v4
        //    \ /    \ /
        //     m1    m2
        //       \  /
        //        m3

        let mut g = Graph::new();
        let v1 = g.var(2);
        let v2 = g.var(3);
        let v3 = g.var(5);
        let v4 = g.var(1);
        let m1 = g.map2(&v1.as_input(), &v2.as_input(), |a, b| a * b);
        let m2 = g.map2(&v3.as_input(), &v4.as_input(), |a, b| a * b);
        let m3 = g.map2(&m1.as_input(), &m2.as_input(), |a, b| a + b);
        assert_eq!(m3.value(), 11);

        g.stablize();
        // assert_eq!(m3.value(), 15);
    }
}
