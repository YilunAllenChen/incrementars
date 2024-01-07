use std::{cell::RefCell, rc::Rc};

use crate::{NodeBehavior, NodeId, NodeInput, NodeValue};

pub type MapHandle<SelfT, In> = Rc<RefCell<Map<SelfT, In>>>;

#[derive(Clone)]
pub struct Map<SelfT, In>
where
    SelfT: Clone,
    In: Clone,
{
    pub id: NodeId,
    value: SelfT,
    parent: Rc<RefCell<dyn NodeInput<In>>>,
    func: fn(In) -> SelfT,
}

impl<SelfT, In> NodeBehavior for Map<SelfT, In>
where
    SelfT: Clone,
    In: Clone,
{
    fn id(&self) -> NodeId {
        self.id
    }
    fn stablize(&mut self) {
        self.value = (self.func)(self.parent.borrow().value());
    }
    fn dirty(&self) -> bool {
        // HACK: this is managed by the graph.
        false
    }
}

impl<SelfT, In> NodeValue<SelfT> for Map<SelfT, In>
where
    SelfT: Clone,
    In: Clone,
{
    fn value(&self) -> SelfT {
        self.value.clone()
    }
}

impl<SelfT, In> NodeInput<SelfT> for Map<SelfT, In>
where
    SelfT: Clone,
    In: Clone,
{
}

impl<SelfT, In> Map<SelfT, In>
where
    SelfT: Clone,
    In: Clone,
{
    pub fn make(
        id: NodeId,
        n1: &Rc<RefCell<dyn NodeInput<In>>>,
        func: fn(In) -> SelfT,
    ) -> Map<SelfT, In> {
        Map {
            id,
            value: (func)(n1.borrow().value()),
            parent: n1.clone(),
            func,
        }
    }
}
