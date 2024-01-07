use std::{cell::RefCell, rc::Rc};

use crate::{NodeBehavior, NodeId, NodeInput, NodeValue};

pub type VarHandle<T> = Rc<RefCell<Var<T>>>;

#[derive(Clone)]
pub struct Var<T>
where
    T: Clone,
{
    pub id: NodeId,
    value: RefCell<T>,
    dirty: bool,
}

impl<T> Var<T>
where
    T: Clone,
{
    pub fn set(&mut self, value: T) {
        *self.value.borrow_mut() = value;
        self.dirty = true;
    }

    pub fn make(id: NodeId, value: T) -> Var<T> {
        Var {
            id,
            value: RefCell::new(value),
            dirty: false,
        }
    }
}

impl<SelfT: Clone> NodeBehavior for Var<SelfT> {
    fn id(&self) -> NodeId {
        self.id
    }
    fn dirty(&self) -> bool {
        self.dirty
    }
    fn stablize(&mut self) {
        self.dirty = false;
    }
}

impl<SelfT: Clone> NodeValue<SelfT> for Var<SelfT> {
    fn value(&self) -> SelfT {
        self.value.borrow().clone()
    }
}

impl<SelfT: Clone> NodeInput<SelfT> for Var<SelfT> {}
