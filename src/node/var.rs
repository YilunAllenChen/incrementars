use std::{cell::RefCell, rc::Rc};

use crate::{NodeBehavior, NodeId, NodeInput, NodeValue};

pub type VarHandle<T> = Rc<RefCell<Var<T>>>;

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
