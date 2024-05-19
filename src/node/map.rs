use std::ops::Deref;
use std::{cell::RefCell, rc::Rc};

use super::traits::{Node, Observable};

pub struct _Map1<I, O> {
    pub id: usize,
    pub height: usize,
    pub value: O,
    pub input: Box<dyn Observable<I>>,
    pub f: fn(I) -> O,
}

impl<I, O> Node for _Map1<I, O> {
    fn id(&self) -> usize {
        self.id
    }
    fn stablize(&mut self) {
        self.value = (self.f)(self.input.observe());
    }
    fn height(&self) -> usize {
        self.height
    }
}

pub struct Map1<I, O> {
    pub node: Rc<RefCell<_Map1<I, O>>>,
}

impl<I, O: Clone> Observable<O> for Map1<I, O> {
    fn observe(&self) -> O {
        let borrowed = self.node.deref().borrow();
        borrowed.value.clone()
    }
    fn height(&self) -> usize {
        self.node.deref().borrow().height
    }
}
