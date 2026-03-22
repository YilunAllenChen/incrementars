use std::{cell::RefCell, rc::Rc};

use super::traits::{MaybeDirty, Node, Observable, StabilizationCallback};
use std::ops::Deref;

pub(crate) struct _Var<T> {
    pub(crate) id: usize,
    pub(crate) depth: i32,
    pub(crate) value: T,
    pub(crate) dirty: bool,
}

impl<T> Node for _Var<T> {
    fn id(&self) -> usize {
        self.id
    }
    fn depth(&self) -> i32 {
        self.depth
    }
    fn stabilize(&mut self) -> Vec<StabilizationCallback> {
        self.dirty = false;
        vec![StabilizationCallback::ValueChanged]
    }
    fn adjust_depth(&mut self, _: i32) {
        panic!("Var depth should not change");
    }
}

impl<T> _Var<T> {
    pub fn new(id: usize, depth: i32, value: T) -> Self {
        Self {
            id,
            depth,
            value,
            dirty: false,
        }
    }
}

/// A variable node. The entry point for feeding values into the graph.
pub struct Var<T> {
    pub(crate) node: Rc<RefCell<_Var<T>>>,
}

impl<T> Clone for Var<T> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
        }
    }
}

impl<T> Var<T> {
    pub fn set(&self, value: T) {
        let mut internal = self.node.deref().borrow_mut();
        internal.value = value;
        internal.dirty = true;
    }
}

impl<T> MaybeDirty for Var<T> {
    fn id(&self) -> usize {
        self.node.deref().borrow().id
    }
    fn is_dirty(&self) -> bool {
        self.node.deref().borrow().dirty
    }
}

impl<T: Clone> Observable<T> for Var<T> {
    fn id(&self) -> usize {
        self.node.deref().borrow().id
    }
    fn observe(&self) -> T {
        self.node.deref().borrow().value.clone()
    }
    fn depth(&self) -> i32 {
        self.node.deref().borrow().depth
    }
}

impl<T: Clone + 'static> Var<T> {
    pub fn as_input(&self) -> Box<Var<T>> {
        Box::new(self.clone())
    }
}
