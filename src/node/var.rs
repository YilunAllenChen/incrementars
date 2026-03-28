use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use super::traits::{IntoInput, MaybeDirty, Node, Observable, StabilizationCallback};
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

/// An input node holding a value of type `T`. Create with [`Incrementars::var`].
///
/// Cloning a `Var` produces a second handle to the same node (cheap reference-count bump).
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
    /// Updates the node's value and marks it dirty. The new value is not visible
    /// to downstream nodes until the next [`Incrementars::stabilize`].
    pub fn set(&self, value: T) {
        let mut internal = self.node.deref().borrow_mut();
        internal.value = value;
        internal.dirty = true;
    }

    /// Borrows the node's current value without cloning it.
    pub fn observe_ref(&self) -> Ref<'_, T> {
        Ref::map(self.node.deref().borrow(), |internal| &internal.value)
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
    /// Returns a boxed clone of this handle, suitable for passing to
    /// [`Incrementars::map`], [`Incrementars::map2`], [`Incrementars::map3`],
    /// [`Incrementars::mapn`], or [`Incrementars::bind`].
    pub fn as_input(&self) -> Box<Var<T>> {
        Box::new(self.clone())
    }
}

impl<T: Clone + 'static> IntoInput<T> for Var<T> {
    fn into_input(self) -> Box<dyn Observable<T>> {
        Box::new(self)
    }
}

impl<T: Clone + 'static> IntoInput<T> for Box<Var<T>> {
    fn into_input(self) -> Box<dyn Observable<T>> {
        self
    }
}
