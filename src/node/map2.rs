use std::{cell::RefCell, rc::Rc};

use crate::{NodeBehavior, NodeId, NodeInput, NodeValue};

use super::traits::NodeInputHandle;

pub type Map2Handle<SelfT, In1, In2> = Rc<RefCell<Map2<SelfT, In1, In2>>>;

#[derive(Clone)]
pub struct Map2<SelfT, In1, In2>
where
    SelfT: Clone,
    In1: Clone,
    In2: Clone,
{
    pub id: NodeId,
    pub value: SelfT,
    pub upstreams: (NodeInputHandle<In1>, NodeInputHandle<In2>),
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
            self.upstreams.0.borrow().value(),
            self.upstreams.1.borrow().value(),
        );
    }
    fn dirty(&self) -> bool {
        // HACK: this is managed by the graph.
        false
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
        n1: &NodeInputHandle<In1>,
        n2: &NodeInputHandle<In2>,
        func: fn(In1, In2) -> SelfT,
    ) -> Map2<SelfT, In1, In2> {
        Map2 {
            id,
            value: (func)(n1.borrow().value(), n2.borrow().value()),
            upstreams: (n1.clone(), n2.clone()),
            func,
        }
    }
}
