use std::{cell::RefCell, rc::Rc};

use crate::{NodeBehavior, NodeId, NodeInput, NodeValue};

use super::traits::NodeInputHandle;

pub type BindHandle<SelfT, In> = Rc<RefCell<Bind2<SelfT, In>>>;

// #[derive(Clone)]
pub struct Bind2<SelfT, In>
where
    SelfT: Clone,
    In: Clone,
{
    pub id: NodeId,
    bound: NodeInputHandle<SelfT>,
    captured_inputs: (NodeInputHandle<SelfT>, NodeInputHandle<SelfT>),
    upstream: NodeInputHandle<In>,
    func: fn(In, (NodeInputHandle<SelfT>, NodeInputHandle<SelfT>)) -> NodeInputHandle<SelfT>,
}

impl<SelfT, In> NodeBehavior for Bind2<SelfT, In>
where
    SelfT: Clone,
    In: Clone,
{
    fn id(&self) -> NodeId {
        self.id
    }
    fn stablize(&mut self) {
        self.bound = (self.func)(self.upstream.borrow().value(), self.captured_inputs.clone());
    }
    fn dirty(&self) -> bool {
        // HACK: this is managed by the graph.
        false
    }
}

impl<SelfT, In> NodeValue<SelfT> for Bind2<SelfT, In>
where
    SelfT: Clone,
    In: Clone,
{
    fn value(&self) -> SelfT {
        self.bound.borrow().value()
    }
}

impl<SelfT, In> NodeInput<SelfT> for Bind2<SelfT, In>
where
    SelfT: Clone,
    In: Clone,
{
}

impl<SelfT, In> Bind2<SelfT, In>
where
    SelfT: Clone,
    In: Clone,
{
    pub fn make(
        id: NodeId,
        n1: &NodeInputHandle<In>,
        func: fn(In, (NodeInputHandle<SelfT>, NodeInputHandle<SelfT>)) -> NodeInputHandle<SelfT>,
        captured_inputs: (NodeInputHandle<SelfT>, NodeInputHandle<SelfT>),
    ) -> Bind2<SelfT, In> {
        Bind2 {
            id,
            bound: (func)(n1.borrow().value(), captured_inputs.clone()),
            upstream: n1.clone(),
            func,
            captured_inputs,
        }
    }
}
