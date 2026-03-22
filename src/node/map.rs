use std::ops::Deref;
use std::{cell::RefCell, rc::Rc};

use super::traits::{Node, Observable, StabilizationCallback};

pub(crate) struct _Map1<I, O> {
    pub(crate) id: usize,
    pub(crate) depth: i32,
    pub(crate) value: O,
    pub(crate) input: Box<dyn Observable<I>>,
    pub(crate) f: Box<dyn Fn(I) -> O>,
}

impl<I: 'static, O: PartialEq + 'static> Node for _Map1<I, O> {
    fn id(&self) -> usize {
        self.id
    }
    fn stabilize(&mut self) -> Vec<StabilizationCallback> {
        let new_value = (self.f)(self.input.observe());
        if new_value == self.value {
            return vec![];
        }
        self.value = new_value;
        vec![StabilizationCallback::ValueChanged]
    }
    fn depth(&self) -> i32 {
        self.depth
    }
    fn adjust_depth(&mut self, new_depth: i32) {
        self.depth = new_depth;
    }
}

/// A node that maps one upstream value through a function. Create with [`Incrementars::map`].
///
/// Cloning a `Map1` produces a second handle to the same node.
pub struct Map1<I, O> {
    pub(crate) node: Rc<RefCell<_Map1<I, O>>>,
}

impl<I, O: Clone> Observable<O> for Map1<I, O> {
    fn id(&self) -> usize {
        self.node.deref().borrow().id
    }
    fn observe(&self) -> O {
        self.node.deref().borrow().value.clone()
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

impl<I, O: Clone + 'static> Map1<I, O> {
    /// Returns a boxed clone of this handle, suitable for passing to
    /// [`Incrementars::map`], [`Incrementars::map2`], or [`Incrementars::bind`].
    pub fn as_input(&self) -> Box<Map1<I, O>> {
        Box::new(self.clone())
    }
}
