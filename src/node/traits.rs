use std::{
    cell::{Ref, RefCell},
    marker::PhantomData,
    rc::Rc,
};

/// How a map node decides whether to propagate a newly computed value downstream.
///
/// `Direct` stores a bare function pointer (zero allocation, devirtualizable by LLVM
/// when the callee is known at monomorphization time, e.g. `PartialEq::eq`).
/// `Custom` stores a heap-allocated closure for caller-supplied predicates.
pub(crate) enum Cutoff<O> {
    Direct(fn(&O, &O) -> bool),
    Custom(Box<dyn Fn(&O, &O) -> bool>),
}

impl<O> Cutoff<O> {
    #[inline]
    pub(crate) fn check(&self, new: &O, old: &O) -> bool {
        match self {
            Cutoff::Direct(f) => f(new, old),
            Cutoff::Custom(f) => f(new, old),
        }
    }
}

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
/// use [`Observable`] and the public wrapper types to read node values and wire nodes together.
pub(crate) trait InternalNode {
    fn stabilize(&mut self) -> StabilizationResult;
    fn depth(&self) -> i32;
    fn adjust_depth(&mut self, new_depth: i32);
    fn teardown(&mut self) {}
}

pub struct ValueState<T> {
    pub(crate) id: usize,
    pub(crate) depth: i32,
    pub(crate) value: T,
}

impl<T> ValueState<T> {
    pub(crate) fn new(id: usize, depth: i32, value: T) -> Self {
        Self { id, depth, value }
    }
}

/// A read-only handle to a node in the incremental graph.
///
/// `Signal<T>` is the common handle type used by graph-building APIs. Cloning it is
/// cheap and creates another handle to the same node.
pub struct Signal<T> {
    pub(crate) state: Rc<RefCell<ValueState<T>>>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl<T> Signal<T> {
    pub(crate) fn new(id: usize, depth: i32, value: T) -> Self {
        Self {
            state: Rc::new(RefCell::new(ValueState::new(id, depth, value))),
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

/// A graph node handle returned by [`Graph::map`](crate::node::Graph::map) and
/// [`Graph::map_with_cutoff`](crate::node::Graph::map_with_cutoff).
pub struct Map1<I, O> {
    pub(crate) inner: Signal<O>,
    pub(crate) _phantom: PhantomData<fn(I) -> O>,
}

/// A graph node handle returned by [`Graph::map2`](crate::node::Graph::map2).
pub struct Map2<I1, I2, O> {
    pub(crate) inner: Signal<O>,
    pub(crate) _phantom: PhantomData<fn(I1, I2) -> O>,
}

/// A graph node handle returned by [`Graph::map3`](crate::node::Graph::map3).
pub struct Map3<I1, I2, I3, O> {
    pub(crate) inner: Signal<O>,
    pub(crate) _phantom: PhantomData<fn(I1, I2, I3) -> O>,
}

/// A graph node handle returned by [`Graph::mapn`](crate::node::Graph::mapn).
pub struct MapN<I, O> {
    pub(crate) inner: Signal<O>,
    pub(crate) _phantom: PhantomData<fn(I) -> O>,
}

/// A graph node handle returned by [`Graph::bind`](crate::node::Graph::bind).
pub struct Bind<I, O> {
    pub(crate) inner: Signal<O>,
    pub(crate) _phantom: PhantomData<fn(I) -> O>,
}

macro_rules! impl_node_handle {
    ($name:ident < $($generics:ident),+ >) => {
        impl<$($generics),+> Clone for $name<$($generics),+> {
            fn clone(&self) -> Self {
                Self {
                    inner: self.inner.clone(),
                    _phantom: PhantomData,
                }
            }
        }

        impl<$($generics),+> $name<$($generics),+> {
            /// Borrows the node's current value without cloning it.
            pub fn observe_ref(&self) -> Ref<'_, O> {
                self.inner.observe_ref()
            }
        }

        impl<$($generics),+> Observable<O> for $name<$($generics),+>
        where
            O: Clone,
        {
            fn id(&self) -> usize {
                self.inner.id()
            }

            fn observe(&self) -> O {
                self.inner.observe()
            }

            fn depth(&self) -> i32 {
                self.inner.depth()
            }
        }

        impl<$($generics),+> IntoInput<O> for $name<$($generics),+>
        where
            O: Clone,
        {
            fn into_input(self) -> Signal<O> {
                self.inner
            }
        }

        impl<$($generics),+> IntoInput<O> for &$name<$($generics),+>
        where
            O: Clone,
        {
            fn into_input(self) -> Signal<O> {
                self.inner.clone()
            }
        }
    };
}

impl_node_handle!(Map1<I, O>);
impl_node_handle!(Map2<I1, I2, O>);
impl_node_handle!(Map3<I1, I2, I3, O>);
impl_node_handle!(MapN<I, O>);
impl_node_handle!(Bind<I, O>);

/// A node whose current value can be read.
pub trait Observable<T> {
    fn id(&self) -> usize;
    /// Returns the node's current value. Does **not** trigger recomputation;
    /// call [`Graph::stabilize`](crate::node::Graph::stabilize) first
    /// to propagate any pending changes.
    fn observe(&self) -> T;
    fn depth(&self) -> i32;
}

impl<T: Clone> Observable<T> for Signal<T> {
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

impl<T> PartialEq for Signal<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

/// Converts a node handle into an owned input accepted by graph-construction APIs.
pub trait IntoInput<T> {
    fn into_input(self) -> Signal<T>;
}

impl<T> IntoInput<T> for Signal<T> {
    fn into_input(self) -> Signal<T> {
        self
    }
}

impl<T> IntoInput<T> for &Signal<T> {
    fn into_input(self) -> Signal<T> {
        self.clone()
    }
}
