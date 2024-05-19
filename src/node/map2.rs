use std::ops::Deref;
use std::{cell::RefCell, rc::Rc};

use super::traits::{Node, Observable};

pub struct _Map2<I1, I2, O> {
    pub id: usize,
    pub height: usize,
    pub value: O,
    pub input1: Box<dyn Observable<I1>>,
    pub input2: Box<dyn Observable<I2>>,
    pub f: fn(I1, I2) -> O,
}

impl<I1, I2, O> Node for _Map2<I1, I2, O> {
    fn id(&self) -> usize {
        self.id
    }
    fn stablize(&mut self) {
        self.value = (self.f)(self.input1.observe(), self.input2.observe());
    }
    fn height(&self) -> usize {
        self.height
    }
}

pub struct Map2<I1, I2, O> {
    pub node: Rc<RefCell<_Map2<I1, I2, O>>>,
}

impl<I1, I2, O: Clone> Observable<O> for Map2<I1, I2, O> {
    fn observe(&self) -> O {
        let borrowed = self.node.deref().borrow();
        borrowed.value.clone()
    }
    fn height(&self) -> usize {
        self.node.deref().borrow().height
    }
}
