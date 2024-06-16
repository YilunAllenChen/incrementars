use std::ops::Deref;
use std::{cell::RefCell, rc::Rc};

use super::traits::{Node, Observable, StablizationCallback};

pub struct _Map2<I1, I2, O> {
    pub id: usize,
    pub depth: i32,
    pub value: O,
    pub input1: Box<dyn Observable<I1>>,
    pub input2: Box<dyn Observable<I2>>,
    pub f: fn(I1, I2) -> O,
}

impl<I1, I2, O> Node for _Map2<I1, I2, O> {
    fn id(&self) -> usize {
        self.id
    }
    fn stablize(&mut self) -> Vec<StablizationCallback> {
        self.value = (self.f)(self.input1.observe(), self.input2.observe());
        vec![StablizationCallback::ValueChanged]
    }
    fn depth(&self) -> i32 {
        self.depth
    }

    fn adjust_depth(&mut self, new_depth: i32) {
        self.depth = new_depth;
    }
}

pub struct Map2<I1, I2, O> {
    pub node: Rc<RefCell<_Map2<I1, I2, O>>>,
}

impl<I1, I2, O: Clone> Observable<O> for Map2<I1, I2, O> {
    fn id(&self) -> usize {
        self.node.deref().borrow().id
    }
    fn observe(&self) -> O {
        let borrowed = self.node.deref().borrow();
        borrowed.value.clone()
    }
    fn depth(&self) -> i32 {
        self.node.deref().borrow().depth
    }
}

impl<I1, I2, O> Clone for Map2<I1, I2, O> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
        }
    }
}
