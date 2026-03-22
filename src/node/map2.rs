use std::ops::Deref;
use std::{cell::RefCell, rc::Rc};

use super::traits::{Node, Observable, StabilizationCallback};

pub(crate) struct _Map2<I1, I2, O> {
    pub(crate) id: usize,
    pub(crate) depth: i32,
    pub(crate) value: O,
    pub(crate) input1: Box<dyn Observable<I1>>,
    pub(crate) input2: Box<dyn Observable<I2>>,
    pub(crate) f: Box<dyn Fn(I1, I2) -> O>,
}

impl<I1: 'static, I2: 'static, O: PartialEq + 'static> Node for _Map2<I1, I2, O> {
    fn id(&self) -> usize {
        self.id
    }
    fn stabilize(&mut self) -> Vec<StabilizationCallback> {
        let new_value = (self.f)(self.input1.observe(), self.input2.observe());
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

/// A node that combines two upstream values through a function. Create with [`Incrementars::map2`].
///
/// Cloning a `Map2` produces a second handle to the same node.
pub struct Map2<I1, I2, O> {
    pub(crate) node: Rc<RefCell<_Map2<I1, I2, O>>>,
}

impl<I1, I2, O: Clone> Observable<O> for Map2<I1, I2, O> {
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

impl<I1, I2, O> Clone for Map2<I1, I2, O> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
        }
    }
}

impl<I1, I2, O: Clone + 'static> Map2<I1, I2, O> {
    /// Returns a boxed clone of this handle, suitable for passing to
    /// [`Incrementars::map`], [`Incrementars::map2`], or [`Incrementars::bind`].
    pub fn as_input(&self) -> Box<Map2<I1, I2, O>> {
        Box::new(self.clone())
    }
}
