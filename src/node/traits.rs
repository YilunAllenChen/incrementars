use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

pub enum StabilizationResult {
    Unchanged,
    Changed,
    Rebound {
        from: usize,
        to: usize,
        value_changed: bool,
    },
}

/// Internal trait implemented by all node types. Not part of the public API;
/// use [`Observable`] and [`Incr`] to read node values and wire nodes together.
pub(crate) trait Node {
    fn stabilize(&mut self) -> StabilizationResult;
    fn depth(&self) -> i32;
    fn adjust_depth(&mut self, new_depth: i32);
    fn teardown(&mut self) {}
}

pub(crate) struct NodeState<T> {
    pub(crate) id: usize,
    pub(crate) depth: i32,
    pub(crate) value: T,
}

impl<T> NodeState<T> {
    pub(crate) fn new(id: usize, depth: i32, value: T) -> Self {
        Self { id, depth, value }
    }
}

/// A read-only handle to a node in the incremental graph.
///
/// `Incr<T>` is the common handle type used by graph-building APIs. Cloning it is
/// cheap and creates another handle to the same node.
pub struct Incr<T> {
    pub(crate) state: Rc<RefCell<NodeState<T>>>,
}

impl<T> Clone for Incr<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl<T> Incr<T> {
    pub(crate) fn new(id: usize, depth: i32, value: T) -> Self {
        Self {
            state: Rc::new(RefCell::new(NodeState::new(id, depth, value))),
        }
    }

    pub(crate) fn id(&self) -> usize {
        self.state.borrow().id
    }

    pub(crate) fn depth(&self) -> i32 {
        self.state.borrow().depth
    }

    /// Borrows the node's current value without cloning it.
    pub fn observe_ref(&self) -> Ref<'_, T> {
        Ref::map(self.state.borrow(), |state| &state.value)
    }
}

/// A node whose current value can be read.
pub trait Observable<T> {
    fn id(&self) -> usize;
    /// Returns the node's current value. Does **not** trigger recomputation;
    /// call [`Incrementars::stabilize`](crate::node::Incrementars::stabilize) first
    /// to propagate any pending changes.
    fn observe(&self) -> T;
    fn depth(&self) -> i32;
}

impl<T: Clone> Observable<T> for Incr<T> {
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

impl<T> PartialEq for Incr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

/// Converts a node handle into an owned input accepted by graph-construction APIs.
pub trait IntoInput<T> {
    fn into_input(self) -> Incr<T>;
}

impl<T> IntoInput<T> for Incr<T> {
    fn into_input(self) -> Incr<T> {
        self
    }
}

impl<T> IntoInput<T> for &Incr<T> {
    fn into_input(self) -> Incr<T> {
        self.clone()
    }
}
