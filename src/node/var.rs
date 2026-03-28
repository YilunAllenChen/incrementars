use std::{
    cell::{Cell, Ref, RefCell},
    rc::Rc,
};

use super::traits::{
    Incr, IntoInput, MaybeDirty, Node, NodeState, Observable, StabilizationCallback,
};

pub(crate) struct _Var<T> {
    pub(crate) state: Rc<RefCell<NodeState<T>>>,
    pub(crate) dirty: Rc<Cell<bool>>,
}

impl<T> Node for _Var<T> {
    fn id(&self) -> usize {
        self.state.borrow().id
    }

    fn depth(&self) -> i32 {
        self.state.borrow().depth
    }

    fn stabilize(&mut self) -> Vec<StabilizationCallback> {
        self.dirty.set(false);
        vec![StabilizationCallback::ValueChanged]
    }

    fn adjust_depth(&mut self, _: i32) {
        panic!("Var depth should not change");
    }
}

impl<T> _Var<T> {
    pub(crate) fn new(state: Rc<RefCell<NodeState<T>>>, dirty: Rc<Cell<bool>>) -> Self {
        Self { state, dirty }
    }
}

/// An input node holding a value of type `T`. Create with [`Incrementars::var`].
///
/// Cloning a `Var` produces a second handle to the same node (cheap reference-count bump).
pub struct Var<T> {
    pub(crate) state: Rc<RefCell<NodeState<T>>>,
    pub(crate) dirty: Rc<Cell<bool>>,
}

impl<T> Clone for Var<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            dirty: self.dirty.clone(),
        }
    }
}

impl<T> Var<T> {
    /// Updates the node's value and marks it dirty. The new value is not visible
    /// to downstream nodes until the next [`Incrementars::stabilize`].
    pub fn set(&self, value: T) {
        self.state.borrow_mut().value = value;
        self.dirty.set(true);
    }

    /// Borrows the node's current value without cloning it.
    pub fn observe_ref(&self) -> Ref<'_, T> {
        Ref::map(self.state.borrow(), |state| &state.value)
    }
}

impl<T> MaybeDirty for Var<T> {
    fn id(&self) -> usize {
        self.state.borrow().id
    }

    fn is_dirty(&self) -> bool {
        self.dirty.get()
    }
}

impl<T: Clone> Observable<T> for Var<T> {
    fn id(&self) -> usize {
        self.state.borrow().id
    }

    fn observe(&self) -> T {
        self.state.borrow().value.clone()
    }

    fn depth(&self) -> i32 {
        self.state.borrow().depth
    }
}

impl<T: Clone> IntoInput<T> for Var<T> {
    fn into_input(self) -> Incr<T> {
        Incr { state: self.state }
    }
}

impl<T: Clone> IntoInput<T> for &Var<T> {
    fn into_input(self) -> Incr<T> {
        Incr {
            state: self.state.clone(),
        }
    }
}
