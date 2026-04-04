use std::{
    cell::{Cell, Ref, RefCell},
    rc::Rc,
};

use super::traits::{InternalNode, IntoInput, Observable, Signal, StabilizationResult, ValueState};

pub(crate) struct VarNode<T> {
    pub(crate) state: Rc<RefCell<ValueState<T>>>,
    pub(crate) dirty: Rc<Cell<bool>>,
}

impl<T> InternalNode for VarNode<T> {
    fn depth(&self) -> i32 {
        self.state.borrow().depth
    }

    fn stabilize(&mut self) -> StabilizationResult {
        self.dirty.set(false);
        StabilizationResult::Changed
    }

    fn adjust_depth(&mut self, _: i32) {
        panic!("Var depth should not change");
    }
}

impl<T> VarNode<T> {
    pub(crate) fn new(state: Rc<RefCell<ValueState<T>>>, dirty: Rc<Cell<bool>>) -> Self {
        Self { state, dirty }
    }
}

/// An input node holding a value of type `T`. Create with [`Graph::var`].
///
/// Cloning a `Var` produces a second handle to the same node (cheap reference-count bump).
pub struct Var<T> {
    pub(crate) state: Rc<RefCell<ValueState<T>>>,
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
    /// to downstream nodes until the next [`Graph::stabilize`].
    pub fn set(&self, value: T) {
        self.state.borrow_mut().value = value;
        self.dirty.set(true);
    }

    /// Updates the node's value only if it differs from the current value,
    /// avoiding unnecessary downstream recomputation.
    pub fn set_if_changed(&self, value: T) where T: PartialEq {
        if self.state.borrow().value != value {
            self.state.borrow_mut().value = value;
            self.dirty.set(true);
        }
    }

    /// Borrows the node's current value without cloning it.
    pub fn observe_ref(&self) -> Ref<'_, T> {
        Ref::map(self.state.borrow(), |state| &state.value)
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
    fn into_input(self) -> Signal<T> {
        Signal { state: self.state }
    }
}

impl<T: Clone> IntoInput<T> for &Var<T> {
    fn into_input(self) -> Signal<T> {
        Signal {
            state: self.state.clone(),
        }
    }
}
