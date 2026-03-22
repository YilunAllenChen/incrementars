pub enum StabilizationCallback {
    ValueChanged,
    DependenciesUpdated { from: Vec<usize>, to: Vec<usize> },
}

/// Internal trait implemented by all node types. Not part of the public API;
/// use [`Observable`] to read node values and pass nodes into the graph.
pub(crate) trait Node {
    fn id(&self) -> usize;
    fn stabilize(&mut self) -> Vec<StabilizationCallback>;
    fn depth(&self) -> i32;
    fn adjust_depth(&mut self, new_depth: i32);
}

/// A node whose current value can be read.
///
/// Implemented by [`Var`](crate::node::Var), [`Map1`](crate::node::Map1),
/// [`Map2`](crate::node::Map2), and [`Bind1`](crate::node::Bind1).
///
/// Pass a `Box<dyn Observable<T>>` (obtained via `.as_input()`) to
/// [`Incrementars::map`](crate::node::Incrementars::map),
/// [`Incrementars::map2`](crate::node::Incrementars::map2), or
/// [`Incrementars::bind`](crate::node::Incrementars::bind) to wire nodes together.
pub trait Observable<T> {
    fn id(&self) -> usize;
    /// Returns the node's current value. Does **not** trigger recomputation;
    /// call [`Incrementars::stabilize`](crate::node::Incrementars::stabilize) first
    /// to propagate any pending changes.
    fn observe(&self) -> T;
    fn depth(&self) -> i32;
}

impl<T> PartialEq for dyn Observable<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

pub(crate) trait MaybeDirty {
    fn id(&self) -> usize;
    fn is_dirty(&self) -> bool;
}
