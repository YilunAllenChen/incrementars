use std::ops::Deref;
use std::{cell::RefCell, rc::Rc};

use super::traits::{Node, Observable, StablizationCallback};

pub struct _Map1<I, O> {
    pub id: usize,
    pub depth: i32,
    pub value: O,
    pub input: Box<dyn Observable<I>>,
    pub f: fn(I) -> O,
}

impl<I, O> Node for _Map1<I, O> {
    fn id(&self) -> usize {
        self.id
    }

    fn stablize(&mut self) -> Vec<StablizationCallback> {
        self.value = (self.f)(self.input.observe());
        vec![StablizationCallback::ValueChanged]
    }
    fn depth(&self) -> i32 {
        self.depth
    }
    fn adjust_depth(&mut self, new_depth: i32) {
        self.depth = new_depth;
    }
}

pub struct Map1<I, O> {
    pub node: Rc<RefCell<_Map1<I, O>>>,
}

impl<I, O: Clone> Observable<O> for Map1<I, O> {
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

impl<I, O> Clone for Map1<I, O> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
        }
    }
}

impl<I, O> Map1<I, O> {
    pub fn as_input(&self) -> Box<Map1<I, O>> {
        Box::new(self.clone())
    }
}
