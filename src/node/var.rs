use std::{cell::RefCell, rc::Rc};

use super::traits::{MaybeDirty, Node, Observable, StablizationCallback};
use std::ops::Deref;

/// Internal representation of a Var node.
pub struct _Var<T> {
    id: usize,
    depth: i32,
    value: T,
    dirty: bool,
}

impl<T> Node for _Var<T> {
    fn id(&self) -> usize {
        self.id
    }
    fn depth(&self) -> i32 {
        self.depth
    }
    fn stablize(&mut self) -> Vec<StablizationCallback> {
        vec![StablizationCallback::ValueChanged]
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
/// A variable node.
pub struct Var<T> {
    pub node: Rc<RefCell<_Var<T>>>,
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
        let borrowed = self.node.deref().borrow();
        borrowed.value.clone()
    }
    fn depth(&self) -> i32 {
        self.node.deref().borrow().depth
    }
}
