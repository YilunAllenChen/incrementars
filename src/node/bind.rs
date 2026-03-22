use std::ops::Deref;
use std::{cell::RefCell, rc::Rc};

use super::traits::{Node, Observable, StabilizationCallback};

pub(crate) struct _Bind1<I, O> {
    pub(crate) id: usize,
    pub(crate) depth: i32,
    pub(crate) value: Box<dyn Observable<O>>,
    pub(crate) input: Box<dyn Observable<I>>,
    pub(crate) f: Box<dyn Fn(I) -> Box<dyn Observable<O>>>,
}

impl<I: 'static, O: 'static> Node for _Bind1<I, O> {
    fn id(&self) -> usize {
        self.id
    }

    fn stabilize(&mut self) -> Vec<StabilizationCallback> {
        let new_value = (self.f)(self.input.observe());
        // Compare by node identity — if we're already pointing at the same node, nothing changed.
        if *self.value == *new_value {
            return vec![];
        }
        let old_id = self.value.id();
        let new_id = new_value.id();
        self.value = new_value;
        vec![
            StabilizationCallback::ValueChanged,
            StabilizationCallback::DependenciesUpdated {
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

/// A node whose input graph structure can change dynamically.
pub struct Bind1<I, O> {
    pub(crate) node: Rc<RefCell<_Bind1<I, O>>>,
}

impl<I, O: Clone> Observable<O> for Bind1<I, O> {
    fn id(&self) -> usize {
        self.node.deref().borrow().id
    }
    fn observe(&self) -> O {
        self.node.deref().borrow().value.observe()
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

impl<I, O: Clone + 'static> Bind1<I, O> {
    pub fn as_input(&self) -> Box<Bind1<I, O>> {
        Box::new(self.clone())
    }
}
