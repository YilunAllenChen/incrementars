use std::ops::Deref;
use std::{cell::RefCell, rc::Rc};

use super::traits::{Node, Observable, StablizationCallback};

pub struct _Bind1<I, O> {
    pub id: usize,
    pub depth: i32,
    pub value: Box<dyn Observable<O>>,
    pub input: Box<dyn Observable<I>>,
    pub f: Box<dyn Fn(I) -> Box<dyn Observable<O>>>,
}

impl<I, O> Node for _Bind1<I, O> {
    fn id(&self) -> usize {
        self.id
    }

    fn stablize(&mut self) -> Vec<StablizationCallback> {
        let new_value = (self.f)(self.input.observe());
        if *self.value == *new_value {
            return vec![];
        }
        let old_id = self.value.id();
        let new_id = new_value.id();

        self.value = new_value;
        vec![
            StablizationCallback::ValueChanged,
            StablizationCallback::DependenciesUpdated {
                from: vec![old_id],
                to: vec![new_id],
            },
        ]
    }
    fn depth(&self) -> i32 {
        self.depth
    }

    fn adjust_depth(&mut self, new_depth: i32) {
        self.depth = new_depth;
    }
}

pub struct Bind1<I, O> {
    pub node: Rc<RefCell<_Bind1<I, O>>>,
}

impl<I, O: Clone> Observable<O> for Bind1<I, O> {
    fn id(&self) -> usize {
        self.node.deref().borrow().id
    }
    fn observe(&self) -> O {
        let borrowed = self.node.deref().borrow();
        borrowed.value.observe()
    }
    fn depth(&self) -> i32 {
        self.node.deref().borrow().depth
    }
}

impl<I, O> Clone for Bind1<I, O> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
        }
    }
}
