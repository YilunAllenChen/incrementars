use std::{cell::RefCell, rc::Rc};

use super::traits::{Node, Observable};
use std::ops::Deref;

/// Internal representation of a Var node.
pub struct _Var<T> {
    id: usize,
    height: usize,
    value: T,
}

impl<T> Node for _Var<T> {
    fn id(&self) -> usize {
        self.id
    }
    fn height(&self) -> usize {
        self.height
    }
    fn stablize(&mut self) {}
}

impl<T> _Var<T> {
    pub fn new(id: usize, height: usize, value: T) -> Self {
        Self { id, height, value }
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
        let mut borrowed = self.node.deref().borrow_mut();
        borrowed.value = value;
    }
}

impl<T: Clone> Observable<T> for Var<T> {
    fn observe(&self) -> T {
        let borrowed = self.node.deref().borrow();
        borrowed.value.clone()
    }
    fn height(&self) -> usize {
        self.node.deref().borrow().height
    }
}
